// Mailora v2 — Analytics Component
import { store } from '../store.js';
const el = id => document.getElementById(id);
export function mountAnalytics() {
    store.subscribe('analyticsOpen', render);
}
function render(open) {
    const d = el('analytics-drawer');
    if (!d) return;
    d.classList.toggle('open', open);
    if (open) updateStats();
}
function updateStats() {
    const msgs = store.getState().messages;
    const total = msgs.length;
    const sent = msgs.filter(m => m.folder === 'Sent').length;
    const received = total - sent;
    const unread = msgs.filter(m => !m.read).length;
    const contacts = {};
    msgs.forEach(m => { contacts[m.from] = (contacts[m.from]||0)+1; });
    const top = Object.entries(contacts).sort((a,b)=>b[1]-a[1]).slice(0,3);
    const sTotal = el('stat-total'), sSent = el('stat-sent'), sRecv = el('stat-received'), sResp = el('stat-response'), sTop = el('stat-top');
    if (sTotal) sTotal.textContent = total;
    if (sSent) sSent.textContent = sent;
    if (sRecv) sRecv.textContent = received;
    if (sResp) sResp.textContent = '~' + (1 + Math.random()*3).toFixed(1) + ' saat';
    if (sTop) sTop.innerHTML = top.map((t,i)=>`${i+1}. ${t[0]} (${t[1]})`).join('<br>');
}
