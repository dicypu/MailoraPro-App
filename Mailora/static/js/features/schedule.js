// Schedule send feature
export function getScheduleTime(inputEl) {
    if (!inputEl || !inputEl.value) return null;
    return new Date(inputEl.value).getTime();
}
export function formatSchedule(ts) {
    return new Date(ts).toLocaleString('tr-TR', { day:'numeric', month:'short', hour:'2-digit', minute:'2-digit' });
}
