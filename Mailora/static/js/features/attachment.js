// Attachment feature
import { store, ACTION } from '../store.js';
import { CONFIG } from '../config.js';
export function addFile(file) {
    if (file.size > CONFIG.maxFileSize) return { error: 'Dosya çok büyük (maks. 10MB)' };
    const s = store.getState();
    if (s.attachments.some(a => a.name === file.name && a.size === file.size)) return { error: 'Zaten ekli' };
    if (s.attachments.length >= CONFIG.maxAttachments) return { error: 'Maksimum dosya sayısı' };
    store.dispatch({ type: ACTION.ADD_ATTACHMENT, payload: file });
    return { success: true };
}
export function removeFile(index) { store.dispatch({ type: ACTION.REMOVE_ATTACHMENT, payload: index }); }
export function clearFiles() { store.dispatch({ type: ACTION.CLEAR_ATTACHMENTS }); }
export function getFileIcon(type) {
    if (type.startsWith('image/')) return '🖼️';
    if (type.includes('pdf')) return '📄';
    if (type.includes('word')||type.includes('document')) return '📝';
    if (type.includes('excel')||type.includes('spreadsheet')) return '📊';
    if (type.includes('zip')||type.includes('archive')) return '📦';
    if (type.includes('audio')) return '🎵';
    if (type.includes('video')) return '🎬';
    return '📎';
}
export function formatSize(b) {
    if (b===0) return '0 B';
    const k=1024, s=['B','KB','MB','GB'], i=Math.floor(Math.log(b)/Math.log(k));
    return parseFloat((b/Math.pow(k,i)).toFixed(1))+' '+s[i];
}
