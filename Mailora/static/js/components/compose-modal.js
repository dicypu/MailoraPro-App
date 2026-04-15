// Mailora v2 — Compose Modal Component
import { store, ACTION } from '../store.js';
import { CONFIG } from '../config.js';
import { dataSource } from '../data-source.js';
const el = id => document.getElementById(id);
export function mountCompose() {
    store.subscribe('composeOpen', render);
    store.subscribe('attachments', renderAttachments);
}
function render(open) {
    const modal = el('compose-modal');
    if (!modal) return;
    modal.style.display = open ? 'flex' : 'none';
    if (open) { el('compose-to')?.focus(); initDropzone(); }
}
function initDropzone() {
    const dz = el('dropzone');
    if (!dz || dz._init) return;
    dz._init = true;
    ['dragenter','dragover','dragleave','drop'].forEach(e => dz.addEventListener(e, ev => { ev.preventDefault(); ev.stopPropagation(); }));
    ['dragenter','dragover'].forEach(e => dz.addEventListener(e, () => dz.classList.add('dragover')));
    ['dragleave','drop'].forEach(e => dz.addEventListener(e, () => dz.classList.remove('dragover')));
    dz.addEventListener('drop', e => handleFiles(e.dataTransfer.files));
}
export function handleFileInput(e) { handleFiles(e.target.files); e.target.value = ''; }
function handleFiles(files) {
    [...files].forEach(f => {
        if (f.size > CONFIG.maxFileSize) { showToast(`⚠️ ${f.name} çok büyük (maks. 10MB)`); return; }
        const s = store.getState();
        if (s.attachments.some(a => a.name === f.name && a.size === f.size)) { showToast(`⚠️ ${f.name} zaten ekli`); return; }
        if (s.attachments.length >= CONFIG.maxAttachments) { showToast('⚠️ Maksimum dosya sayısına ulaşıldı'); return; }
        store.dispatch({ type: ACTION.ADD_ATTACHMENT, payload: f });
    });
}
function renderAttachments(atts) {
    const c = el('attachment-preview');
    if (!c) return;
    c.innerHTML = atts.map((f, i) => {
        const icon = f.type.startsWith('image/')?'🖼️':f.type.includes('pdf')?'📄':f.type.includes('word')?'📝':f.type.includes('excel')?'📊':'📎';
        const size = f.size<1024?f.size+'B':f.size<1048576?(f.size/1024).toFixed(1)+'KB':(f.size/1048576).toFixed(1)+'MB';
        return `<div class="att-chip"><span>${icon}</span><span class="att-name">${f.name}</span><span class="att-sz">${size}</span><button class="att-rm" data-i="${i}">✕</button></div>`;
    }).join('');
    c.querySelectorAll('.att-rm').forEach(b => b.onclick = () => store.dispatch({ type: ACTION.REMOVE_ATTACHMENT, payload: parseInt(b.dataset.i) }));
}
export async function sendEmail() {
    const to = el('compose-to')?.value, subj = el('compose-subject')?.value, body = el('compose-body')?.value;
    if (!to) { showToast('⚠️ Alıcı gerekli'); return; }
    const scheduled = el('schedule-check')?.checked;
    const scheduleTime = el('schedule-time')?.value;
    if (scheduled && scheduleTime) { showToast(`📅 ${new Date(scheduleTime).toLocaleString('tr-TR')} için zamanlandı`); closeCompose(); return; }
    // Undo send
    const toast = document.createElement('div'); toast.className='toast undo';
    toast.innerHTML='Gönderiliyor... <button class="undo-btn" id="undo-send">İptal</button>';
    document.body.appendChild(toast);
    const timeout = setTimeout(async () => {
        toast.remove();
        try {
            const s = store.getState();
            await dataSource.sendMessage({ to, subject: subj, body, accountId: s.selectedAccountId, attachments: s.attachments });
            showToast('✓ E-posta gönderildi!');
        } catch(e) { showToast('❌ Gönderilemedi: ' + e.message); }
        closeCompose();
    }, 5000);
    el('undo-send')?.addEventListener('click', () => { clearTimeout(timeout); toast.remove(); showToast('↩️ Gönderim iptal edildi'); });
}
function closeCompose() {
    store.dispatch({ type: ACTION.TOGGLE_COMPOSE });
    ['compose-to','compose-subject','compose-body'].forEach(id => { const e=el(id); if(e) e.value=''; });
}
function showToast(msg) {
    const t = document.createElement('div'); t.className='toast'; t.textContent=msg;
    document.body.appendChild(t); setTimeout(()=>t.remove(), 3000);
}
export { closeCompose, showToast };
