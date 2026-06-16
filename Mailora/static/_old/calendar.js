/**
 * Mailora Hub - Calendar Logic (Vanilla JS)
 * Implements Month, Week, Day views with CSS Grid.
 */

class CalendarApp {
    constructor() {
        this.currentDate = new Date();
        this.selectedView = 'month'; // month, week, day
        this.calendars = [];
        this.events = [];
        this.accounts = [];
        this.notifiedEvents = new Set();

        this.init();
    }

    async init() {
        this.cacheDOM();
        this.bindEvents();
        await this.loadAccounts();
        this.requestNotificationPermission();
        this.startReminderCheck();
        this.render();
    }

    requestNotificationPermission() {
        if ("Notification" in window && Notification.permission !== "denied") {
            Notification.requestPermission();
        }
    }

    startReminderCheck() {
        setInterval(() => this.checkReminders(), 60000); // Check every minute
    }

    checkReminders() {
        const now = new Date();
        this.events.forEach(ev => {
            if (!ev.dtstart || this.notifiedEvents.has(ev.id)) return;
            const eventTime = new Date(ev.dtstart);
            const diff = eventTime - now;

            // Notify if event starts in less than 15 minutes and hasn't passed
            if (diff > 0 && diff < 15 * 60 * 1000) {
                this.showNotification(ev);
                this.notifiedEvents.add(ev.id);
            }
        });
    }

    showNotification(ev) {
        if ("Notification" in window && Notification.permission === "granted") {
            new Notification("Calendar Reminder", {
                body: `${ev.summary} starts soon at ${new Date(ev.dtstart).toLocaleTimeString()}`,
                icon: 'https://ui-avatars.com/api/?name=C&background=6366f1&color=fff'
            });
        }
    }

    cacheDOM() {
        this.grid = document.getElementById('calendarGrid');
        this.periodTitle = document.getElementById('currentPeriod');
        this.prevBtn = document.getElementById('prevPeriod');
        this.nextBtn = document.getElementById('nextPeriod');
        this.todayBtn = document.getElementById('todayBtn');
        this.viewTabs = document.querySelectorAll('.tab-item');
        this.addEventBtn = document.getElementById('addEventBtn');
        this.eventModal = document.getElementById('eventModal');
        this.eventForm = document.getElementById('eventForm');
        this.calendarsContainer = document.getElementById('calendarsContainer');
    }

    bindEvents() {
        this.prevBtn.onclick = () => this.navigate(-1);
        this.nextBtn.onclick = () => this.navigate(1);
        this.todayBtn.onclick = () => {
            this.currentDate = new Date();
            this.render();
        };

        this.viewTabs.forEach(tab => {
            tab.onclick = (e) => {
                this.viewTabs.forEach(t => t.classList.remove('active'));
                tab.classList.add('active');
                this.selectedView = tab.dataset.view;
                this.render();
            };
        });

        this.addEventBtn.onclick = () => this.openModal();
        document.querySelector('.close-modal').onclick = () => this.closeModal();
        document.getElementById('cancelEvent').onclick = () => this.closeModal();

        this.eventForm.onsubmit = (e) => this.handleFormSubmit(e);

        // Add export button listener if exists
        document.getElementById('exportIcsBtn')?.addEventListener('click', () => this.exportCurrentMonth());
    }

    exportCurrentMonth() {
        if (this.events.length === 0) return;
        let ics = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//MailoraHub//Calendar//EN\r\n";
        this.events.forEach(ev => {
            ics += "BEGIN:VEVENT\r\n";
            ics += `UID:${ev.uid || Math.random().toString(36).substr(2, 9)}\r\n`;
            ics += `SUMMARY:${ev.summary}\r\n`;
            if (ev.dtstart) ics += `DTSTART:${ev.dtstart.replace(/[-:]/g, '')}\r\n`;
            if (ev.dtend) ics += `DTEND:${ev.dtend.replace(/[-:]/g, '')}\r\n`;
            if (ev.location) ics += `LOCATION:${ev.location}\r\n`;
            if (ev.description) ics += `DESCRIPTION:${ev.description}\r\n`;
            ics += "END:VEVENT\r\n";
        });
        ics += "END:VCALENDAR\r\n";

        const blob = new Blob([ics], { type: 'text/calendar' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `calendar_export_${new Date().toISOString().split('T')[0]}.ics`;
        a.click();
    }

    async loadAccounts() {
        try {
            const res = await fetch('/accounts');
            const data = await res.json();
            if (data.success) {
                this.accounts = data.data;
                await this.loadCalendars();
            }
        } catch (e) { console.error("Load accounts failed", e); }
    }

    async loadCalendars() {
        const containers = [];
        for (const acc of this.accounts) {
            try {
                const res = await fetch(`/calendar/accounts/${acc.id}/calendars`);
                const data = await res.json();
                if (data.success) {
                    this.calendars.push(...data.data.map(c => ({ ...c, accountEmail: acc.email })));
                }
            } catch (e) { console.warn(`Calendar fetch failed for ${acc.email}`, e); }
        }
        this.renderCalendarList();
        if (this.calendars.length === 0) {
            this.calendarsContainer.innerHTML = '<p class="text-muted" style="font-size:0.8rem; padding:10px;">No calendars found. Please check account settings.</p>';
        }
        await this.loadEvents();
    }

    async loadEvents() {
        this.events = [];
        for (const cal of this.calendars) {
            try {
                const res = await fetch(`/calendar/accounts/${cal.account_id}/calendars/${cal.id}/events`);
                const data = await res.json();
                if (data.success) {
                    this.events.push(...data.data);
                }
            } catch (e) { console.error("Load events failed", e); }
        }
        this.render();
    }

    renderCalendarList() {
        if (this.calendars.length === 0) return;
        this.calendarsContainer.innerHTML = this.calendars.map(cal => `
            <div class="cal-entry" title="${cal.accountEmail}">
                <span class="cal-color" style="background: ${cal.color || '#6366f1'}"></span>
                <span class="cal-name">${cal.display_name}</span>
            </div>
        `).join('');
    }

    navigate(dir) {
        if (this.selectedView === 'month') {
            this.currentDate.setMonth(this.currentDate.getMonth() + dir);
        } else if (this.selectedView === 'week') {
            this.currentDate.setDate(this.currentDate.getDate() + (dir * 7));
        } else {
            this.currentDate.setDate(this.currentDate.getDate() + dir);
        }
        this.render();
    }

    render() {
        this.updateHeader();
        if (this.selectedView === 'month') this.renderMonthView();
        else if (this.selectedView === 'week') this.renderWeekView();
        else this.renderDayView();
    }

    updateHeader() {
        const options = { month: 'long', year: 'numeric' };
        if (this.selectedView === 'day') options.day = 'numeric';
        this.periodTitle.innerText = this.currentDate.toLocaleDateString('en-US', options);
    }

    renderMonthView() {
        this.grid.className = 'calendar-grid month-view';
        this.grid.innerHTML = `
            <div class="grid-header">Sun</div><div class="grid-header">Mon</div>
            <div class="grid-header">Tue</div><div class="grid-header">Wed</div>
            <div class="grid-header">Thu</div><div class="grid-header">Fri</div>
            <div class="grid-header">Sat</div>
        `;

        const year = this.currentDate.getFullYear();
        const month = this.currentDate.getMonth();
        const firstDay = new Date(year, month, 1).getDay();
        const daysInMonth = new Date(year, month + 1, 0).getDate();

        // Prev month buffer
        const prevDaysInMonth = new Date(year, month, 0).getDate();
        for (let i = firstDay - 1; i >= 0; i--) {
            this.grid.appendChild(this.createDayCell(prevDaysInMonth - i, true));
        }

        // Current month
        const today = new Date();
        for (let i = 1; i <= daysInMonth; i++) {
            const isToday = today.getDate() === i && today.getMonth() === month && today.getFullYear() === year;
            const cell = this.createDayCell(i, false, isToday);

            // Add events
            const dateStr = `${year}-${String(month + 1).padStart(2, '0')}-${String(i).padStart(2, '0')}`;
            const dayEvents = this.events.filter(ev => ev.dtstart && ev.dtstart.startsWith(dateStr));
            dayEvents.forEach(ev => {
                const evEl = document.createElement('div');
                evEl.className = 'event-item';
                evEl.innerText = ev.summary;
                evEl.onclick = (e) => { e.stopPropagation(); this.openModal(ev); };
                cell.appendChild(evEl);
            });

            this.grid.appendChild(cell);
        }
    }

    createDayCell(num, isOther, isToday) {
        const div = document.createElement('div');
        div.className = `calendar-day ${isOther ? 'other-month' : ''} ${isToday ? 'today' : ''}`;
        div.innerHTML = `<span class="day-number">${num}</span>`;
        if (!isOther) {
            div.onclick = () => {
                const d = new Date(this.currentDate);
                d.setDate(num);
                this.openModal(null, d);
            }
        }
        return div;
    }

    renderWeekView() {
        this.grid.className = 'calendar-grid week-view';
        this.grid.innerHTML = '<div class="coming-soon">Week View coming in Sprint 3</div>';
    }

    renderDayView() {
        this.grid.className = 'calendar-grid day-view';
        this.grid.innerHTML = '<div class="coming-soon">Day View coming in Sprint 3</div>';
    }

    openModal(event = null, date = null) {
        this.eventModal.classList.add('active');
        if (event) {
            document.querySelector('.modal-header h3').innerText = 'Edit Event';
            document.getElementById('eventTitle').value = event.summary;
            document.getElementById('eventLocation').value = event.location || '';
            document.getElementById('eventDescription').value = event.description || '';
            if (event.dtstart) document.getElementById('startDate').value = event.dtstart.split('T')[0];
            if (event.dtend) document.getElementById('endDate').value = event.dtend.split('T')[0];
        } else {
            this.eventForm.reset();
            document.querySelector('.modal-header h3').innerText = 'New Event';
            const d = date || new Date();
            const dStr = d.toISOString().split('T')[0];
            document.getElementById('startDate').value = dStr;
            document.getElementById('endDate').value = dStr;
        }

        // Fill calendar select
        const sel = document.getElementById('calendarSelect');
        sel.innerHTML = this.calendars.map(c => `<option value="${c.id}" data-acc="${c.account_id}">${c.display_name} (${c.accountEmail})</option>`).join('');
    }

    closeModal() {
        this.eventModal.classList.remove('active');
    }

    async handleFormSubmit(e) {
        e.preventDefault();
        const sel = document.getElementById('calendarSelect');
        const opt = sel.options[sel.selectedIndex];

        if (!opt) {
            alert("Please select a calendar first. If the list is empty, wait for synchronization or check your account settings.");
            return;
        }

        const account_id = opt.dataset.acc;
        const calendar_id = sel.value;

        const payload = {
            summary: document.getElementById('eventTitle').value,
            description: document.getElementById('eventDescription').value,
            location: document.getElementById('eventLocation').value,
            dtstart: document.getElementById('startDate').value + 'T09:00:00Z', // Default time
            dtend: document.getElementById('endDate').value + 'T10:00:00Z',
            is_all_day: false,
            timezone: 'UTC',
            rrule: document.getElementById('eventRRule').value || null
        };

        try {
            console.log("Submitting event to account:", account_id, "calendar:", calendar_id);
            const res = await fetch(`/calendar/accounts/${account_id}/calendars/${calendar_id}/events`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            const data = await res.json();
            if (data.success) {
                console.log("Event saved successfully");
                this.closeModal();
                await this.loadEvents();
            } else {
                alert("Save failed: " + (data.error || "Unknown error"));
            }
        } catch (err) {
            console.error("Save event fetch error:", err);
            alert("Network error: Could not save event.");
        }
    }
}

document.addEventListener('DOMContentLoaded', () => new CalendarApp());
