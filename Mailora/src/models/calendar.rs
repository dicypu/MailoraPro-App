use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Calendar {
    pub id: String,
    pub account_id: String,
    pub url: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub ctag: Option<String>,
    pub sync_token: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CalendarEvent {
    pub id: String,
    pub calendar_id: String,
    pub uid: String,
    pub href: String,
    pub etag: Option<String>,
    pub raw_ical: String,
    
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub is_all_day: i64,
    pub timezone: Option<String>,
    
    pub rrule: Option<String>,
    pub status: Option<String>,
    
    pub sync_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventAttendee {
    pub id: String,
    pub event_id: String,
    pub email: String,
    pub cn: Option<String>,
    pub partstat: String,
    pub is_organizer: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventAlarm {
    pub id: String,
    pub event_id: String,
    pub action: String,
    pub trigger_text: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFull {
    #[serde(flatten)]
    pub event: CalendarEvent,
    pub attendees: Vec<EventAttendee>,
    pub alarms: Vec<EventAlarm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRequest {
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart: String,
    pub dtend: String,
    pub is_all_day: bool,
    pub timezone: Option<String>,
    pub rrule: Option<String>,
    pub attendees: Option<Vec<AttendeeRequest>>,
    pub alarms: Option<Vec<AlarmRequest>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendeeRequest {
    pub email: String,
    pub cn: Option<String>,
    pub partstat: Option<String>, // DEFAULT: NEEDS-ACTION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmRequest {
    pub action: String, // DISPLAY, AUDIO
    pub trigger_text: String, // -PT15M
}
