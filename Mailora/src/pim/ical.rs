use tracing::warn;

#[derive(Debug, Default, serde::Serialize)]
pub struct ParsedICal {
    pub prodid: Option<String>,
    pub version: Option<String>,
    pub events: Vec<ParsedEvent>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ParsedEvent {
    pub uid: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart: Option<String>, // includes logic for value=date for all day
    pub dtend: Option<String>,
    pub tz_id: Option<String>,
    pub rrule: Option<String>,
    pub status: Option<String>,
    pub is_all_day: bool,
    
    pub attendees: Vec<ParsedAttendee>,
    pub alarms: Vec<ParsedAlarm>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ParsedAttendee {
    pub email: String,
    pub cn: Option<String>,
    pub partstat: String, // ACCEPTED, DECLINED, NEEDS-ACTION...
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ParsedAlarm {
    pub action: String, // AUDIO, DISPLAY, EMAIL
    pub trigger: String, // -PT15M
    pub description: Option<String>,
}

pub fn parse_ical(raw: &str) -> Option<ParsedICal> {
    let unfolded = unfold_lines(raw);
    let mut calendar = ParsedICal::default();
    
    let mut current_event: Option<ParsedEvent> = None;
    let mut current_alarm: Option<ParsedAlarm> = None;
    let mut in_calendar = false;

    for line in unfolded.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() < 2 { continue; }

        let prop_and_params = parts[0];
        let val = parts[1];

        let p_parts: Vec<&str> = prop_and_params.split(';').collect();
        let prop_name = p_parts[0].to_uppercase();

        match prop_name.as_str() {
            "BEGIN" => {
                match val.to_uppercase().as_str() {
                    "VCALENDAR" => in_calendar = true,
                    "VEVENT" => {
                        if current_event.is_some() {
                           // Push previous
                           calendar.events.push(current_event.take().unwrap()); 
                        }
                        current_event = Some(ParsedEvent::default());
                    },
                    "VALARM" => {
                        if current_alarm.is_some() && current_event.is_some() {
                            current_event.as_mut().unwrap().alarms.push(current_alarm.take().unwrap());
                        }
                        current_alarm = Some(ParsedAlarm::default());
                    },
                    _ => {}
                }
            },
            "END" => {
                match val.to_uppercase().as_str() {
                    "VEVENT" => {
                        if let Some(mut ev) = current_event.take() {
                            if let Some(al) = current_alarm.take() {
                                ev.alarms.push(al);
                            }
                            if ev.uid.is_some() {
                                calendar.events.push(ev);
                            } else {
                                warn!("Skipped VEVENT without UID");
                            }
                        }
                    },
                    "VALARM" => {
                        if let Some(al) = current_alarm.take() {
                            if let Some(ev) = current_event.as_mut() {
                                ev.alarms.push(al);
                            }
                        }
                    },
                    "VCALENDAR" => in_calendar = false,
                    _ => {}
                }
            },
            "PRODID" => if in_calendar && current_event.is_none() { calendar.prodid = Some(val.to_string()); },
            "VERSION" => if in_calendar && current_event.is_none() { calendar.version = Some(val.to_string()); },
            
            _ => {
                // Event scope
                if let Some(ref mut ev) = current_event {
                    // Check if inside alarm
                    if let Some(ref mut al) = current_alarm {
                        match prop_name.as_str() {
                            "ACTION" => al.action = val.to_uppercase(),
                            "TRIGGER" => al.trigger = val.to_string(), // Keep original format
                            "DESCRIPTION" => al.description = Some(decode_text(val)),
                            _ => {}
                        }
                    } else {
                        // Regular Event Property
                        match prop_name.as_str() {
                            "UID" => ev.uid = Some(val.to_string()),
                            "SUMMARY" => ev.summary = Some(decode_text(val)),
                            "DESCRIPTION" => ev.description = Some(decode_text(val)),
                            "LOCATION" => ev.location = Some(decode_text(val)),
                            "STATUS" => ev.status = Some(val.to_uppercase()),
                            "RRULE" => ev.rrule = Some(val.to_string()), // We store raw RRULE for now
                            "DTSTART" | "DTEND" => {
                                let (is_all_day, tzid) = parse_date_params(&p_parts[1..]);
                                if prop_name == "DTSTART" {
                                    ev.dtstart = Some(val.to_string());
                                    ev.is_all_day = is_all_day;
                                    if let Some(tz) = tzid {
                                        ev.tz_id = Some(tz);
                                    }
                                } else {
                                    ev.dtend = Some(val.to_string());
                                }
                            },
                            "ATTENDEE" => {
                                if let Some(att) = parse_attendee(val, &p_parts[1..]) {
                                    ev.attendees.push(att);
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    
    // In case the file ended unexpectedly without END:VCALENDAR
    if let Some(mut ev) = current_event.take() {
        if let Some(al) = current_alarm.take() {
            ev.alarms.push(al);
        }
        if ev.uid.is_some() {
            calendar.events.push(ev);
        }
    }

    if calendar.events.is_empty() && calendar.prodid.is_none() {
        return None;
    }

    Some(calendar)
}

fn unfold_lines(input: &str) -> String {
    input
        .replace("\r\n ", "")
        .replace("\r\n\t", "")
        .replace("\n ", "")
        .replace("\n\t", "")
}

fn decode_text(val: &str) -> String {
    val.replace("\\n", "\n")
       .replace("\\N", "\n")
       .replace("\\,", ",")
       .replace("\\;", ";")
       .replace("\\\\", "\\")
}

// Returns (is_all_day, Option<TZID>)
fn parse_date_params(params: &[&str]) -> (bool, Option<String>) {
    let mut is_all_day = false;
    let mut tzid = None;
    
    for p in params {
        let kv: Vec<&str> = p.splitn(2, '=').collect();
        if kv.len() == 2 {
            let key = kv[0].trim().to_uppercase();
            let val = kv[1].trim();
            if key == "VALUE" && val.to_uppercase() == "DATE" {
                is_all_day = true;
            } else if key == "TZID" {
                tzid = Some(val.to_string());
            }
        }
    }
    (is_all_day, tzid)
}

fn parse_attendee(val: &str, params: &[&str]) -> Option<ParsedAttendee> {
    let email = val.trim_start_matches("mailto:").trim_start_matches("MAILTO:").to_string();
    if email.is_empty() { return None; }
    
    let mut cn = None;
    let mut partstat = "NEEDS-ACTION".to_string();
    
    for p in params {
        let kv: Vec<&str> = p.splitn(2, '=').collect();
        if kv.len() == 2 {
            let key = kv[0].trim().to_uppercase();
            let val = kv[1].trim().trim_matches('"');
            if key == "CN" {
                cn = Some(decode_text(val));
            } else if key == "PARTSTAT" {
                partstat = val.to_uppercase();
            }
        }
    }
    
    Some(ParsedAttendee {
        email,
        cn,
        partstat,
    })
}

// Basic serializer for simple event creation (Not full RFC 5545 compliance for all edge cases)
pub fn serialize_ical(
    uid: &str,
    summary: &str,
    description: Option<&str>,
    location: Option<&str>,
    dtstart: &str,
    dtend: &str,
    is_all_day: bool,
    timezone: Option<&str>,
    rrule: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Mailora Hub//EN\r\n");
    out.push_str("BEGIN:VEVENT\r\n");
    
    out.push_str(&format!("UID:{}\r\n", uid));
    out.push_str(&format!("DTSTAMP:{}\r\n", chrono::Utc::now().format("%Y%m%dT%H%M%SZ")));
    
    out.push_str(&format!("SUMMARY:{}\r\n", encode_text(summary)));
    
    if let Some(desc) = description {
        out.push_str(&fold_line(&format!("DESCRIPTION:{}", encode_text(desc))));
        out.push_str("\r\n");
    }
    if let Some(loc) = location {
        out.push_str(&fold_line(&format!("LOCATION:{}", encode_text(loc))));
        out.push_str("\r\n");
    }
    
    if is_all_day {
        out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", dtstart.replace("-", "")));
        out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", dtend.replace("-", "")));
    } else {
        if let Some(tz) = timezone {
            out.push_str(&format!("DTSTART;TZID={}:{}\r\n", tz, dtstart.replace("-", "").replace(":", "")));
            out.push_str(&format!("DTEND;TZID={}:{}\r\n", tz, dtend.replace("-", "").replace(":", "")));
        } else {
            // Assume UTC
            out.push_str(&format!("DTSTART:{}\r\n", dtstart.replace("-", "").replace(":", "")));
            out.push_str(&format!("DTEND:{}\r\n", dtend.replace("-", "").replace(":", "")));
        }
    }
    
    if let Some(rule) = rrule {
        out.push_str(&format!("RRULE:{}\r\n", rule));
    }
    
    out.push_str("END:VEVENT\r\n");
    out.push_str("END:VCALENDAR\r\n");
    out
}

fn encode_text(val: &str) -> String {
    val.replace("\\", "\\\\")
       .replace(";", "\\;")
       .replace(",", "\\,")
       .replace("\n", "\\n")
}

fn fold_line(line: &str) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    let mut line_len = 0;
    
    while let Some(c) = chars.next() {
        let char_len = c.len_utf8();
        if line_len + char_len > 75 {
            result.push_str("\r\n ");
            line_len = 1; // 1 space
        }
        result.push(c);
        line_len += char_len;
    }
    result
}
