export const MSG_STATE = { NORMAL: 'normal', PINNED: 'pinned', SNOOZED: 'snoozed' };
export const ACTION = {
    SET_ACCOUNTS: 'SET_ACCOUNTS', SET_MESSAGES: 'SET_MESSAGES', SET_FOLDERS: 'SET_FOLDERS',
    PIN_MESSAGE: 'PIN_MESSAGE', UNPIN_MESSAGE: 'UNPIN_MESSAGE',
    SNOOZE_MESSAGE: 'SNOOZE_MESSAGE', UNSNOOZE_MESSAGE: 'UNSNOOZE_MESSAGE',
    MARK_IMPORTANT: 'MARK_IMPORTANT', MARK_READ: 'MARK_READ', DELETE_MESSAGE: 'DELETE_MESSAGE',
    SELECT_MESSAGE: 'SELECT_MESSAGE', SELECT_ACCOUNT: 'SELECT_ACCOUNT', SELECT_FOLDER: 'SELECT_FOLDER',
    SET_SEARCH: 'SET_SEARCH', TOGGLE_FOCUS: 'TOGGLE_FOCUS', TOGGLE_THEME: 'TOGGLE_THEME',
    TOGGLE_COMPOSE: 'TOGGLE_COMPOSE', TOGGLE_ANALYTICS: 'TOGGLE_ANALYTICS',
    ADD_ATTACHMENT: 'ADD_ATTACHMENT', REMOVE_ATTACHMENT: 'REMOVE_ATTACHMENT', CLEAR_ATTACHMENTS: 'CLEAR_ATTACHMENTS',
    UPDATE_MESSAGES_AI: 'UPDATE_MESSAGES_AI',
};
function initState() {
    return { accounts: [], messages: [], folders: ['Inbox','Sent','Drafts','Spam','Trash'],
        selectedAccountId: null, selectedFolder: 'Inbox', selectedMessageId: null,
        searchQuery: '', focusMode: false, theme: localStorage.getItem('mailora-theme')||'dark',
        composeOpen: false, analyticsOpen: false, attachments: [] };
}
function getMsgState(m) {
    if (m.snoozed && m.snoozeUntil > Date.now()) return MSG_STATE.SNOOZED;
    if (m.pinned) return MSG_STATE.PINNED;
    return MSG_STATE.NORMAL;
}
function transition(msg, target, p={}) {
    if (target === MSG_STATE.PINNED) { msg.snoozed = false; msg.snoozeUntil = null; msg.pinned = true; }
    else if (target === MSG_STATE.SNOOZED) { msg.pinned = false; msg.snoozed = true; msg.snoozeUntil = p.until||(Date.now()+3600000); }
    else { msg.pinned = false; msg.snoozed = false; msg.snoozeUntil = null; }
}
function norm(m) { return {...m, pinned:m.pinned||false, snoozed:m.snoozed||false, snoozeUntil:m.snoozeUntil||null, important:m.important||false, read:m.read||false, isNewsletter:m.isNewsletter||false}; }
function reducer(s, a) {
    const n = {...s};
    switch(a.type) {
        case ACTION.SET_ACCOUNTS: n.accounts=a.payload; if(!n.selectedAccountId&&a.payload.length) n.selectedAccountId=a.payload[0].id; break;
        case ACTION.SET_MESSAGES: n.messages=a.payload.map(norm); break;
        case ACTION.SET_FOLDERS: n.folders=a.payload; break;
        case ACTION.PIN_MESSAGE: n.messages=n.messages.map(m=>{if(m.id!==a.payload)return m;const c={...m};transition(c,MSG_STATE.PINNED);return c;}); break;
        case ACTION.UNPIN_MESSAGE: n.messages=n.messages.map(m=>m.id===a.payload?{...m,pinned:false}:m); break;
        case ACTION.SNOOZE_MESSAGE: n.messages=n.messages.map(m=>{if(m.id!==a.payload.id)return m;const c={...m};transition(c,MSG_STATE.SNOOZED,{until:a.payload.until});return c;}); break;
        case ACTION.UNSNOOZE_MESSAGE: n.messages=n.messages.map(m=>m.id===a.payload?{...m,snoozed:false,snoozeUntil:null}:m); break;
        case ACTION.MARK_IMPORTANT: n.messages=n.messages.map(m=>m.id===a.payload?{...m,important:!m.important}:m); break;
        case ACTION.MARK_READ: n.messages=n.messages.map(m=>m.id===a.payload?{...m,read:true}:m); break;
        case ACTION.DELETE_MESSAGE: n.messages=n.messages.filter(m=>m.id!==a.payload); if(n.selectedMessageId===a.payload) n.selectedMessageId=null; break;
        case ACTION.SELECT_MESSAGE: n.selectedMessageId=a.payload; break;
        case ACTION.SELECT_ACCOUNT: n.selectedAccountId=a.payload; break;
        case ACTION.SELECT_FOLDER: n.selectedFolder=a.payload; n.selectedMessageId=null; break;
        case ACTION.SET_SEARCH: n.searchQuery=a.payload; break;
        case ACTION.TOGGLE_FOCUS: n.focusMode=!n.focusMode; break;
        case ACTION.TOGGLE_THEME: n.theme=n.theme==='dark'?'light':'dark'; localStorage.setItem('mailora-theme',n.theme); document.documentElement.setAttribute('data-theme',n.theme); break;
        case ACTION.TOGGLE_COMPOSE: n.composeOpen=!n.composeOpen; if(!n.composeOpen) n.attachments=[]; break;
        case ACTION.TOGGLE_ANALYTICS: n.analyticsOpen=!n.analyticsOpen; break;
        case ACTION.ADD_ATTACHMENT: n.attachments=[...n.attachments,a.payload]; break;
        case ACTION.REMOVE_ATTACHMENT: n.attachments=n.attachments.filter((_,i)=>i!==a.payload); break;
        case ACTION.CLEAR_ATTACHMENTS: n.attachments=[]; break;
        case ACTION.UPDATE_MESSAGES_AI: n.messages=n.messages.map(m=>a.payload[m.id]?{...m,...a.payload[m.id]}:m); break;
        default: return s;
    }
    return n;
}
class Store {
    constructor() { this._state=initState(); this._subs=new Map(); this._prev=null; }
    getState() { return this._state; }
    dispatch(a) { this._prev=this._state; this._state=reducer(this._state,a); this._wake(); this._notify(); }
    subscribe(key,cb) { if(!this._subs.has(key)) this._subs.set(key,new Set()); this._subs.get(key).add(cb); return ()=>this._subs.get(key)?.delete(cb); }
    _notify() { for(const[k,cbs]of this._subs){if(!this._prev||this._prev[k]!==this._state[k]){for(const cb of cbs){try{cb(this._state[k],this._prev?.[k]);}catch(e){console.error(k,e);}}}} }
    _wake() { const now=Date.now();let ch=false;const ms=this._state.messages.map(m=>{if(m.snoozed&&m.snoozeUntil&&m.snoozeUntil<=now){ch=true;return{...m,snoozed:false,snoozeUntil:null};}return m;});if(ch)this._state={...this._state,messages:ms}; }
    getVisibleMessages() {
        const s=this._state; let ms=s.messages.filter(m=>!m.snoozed||(m.snoozeUntil&&m.snoozeUntil<=Date.now()));
        if(s.focusMode) ms=ms.filter(m=>m.pinned||m.important);
        if(s.searchQuery){const q=s.searchQuery.toLowerCase();ms=ms.filter(m=>m.from?.toLowerCase().includes(q)||m.subject?.toLowerCase().includes(q)||m.preview?.toLowerCase().includes(q));}
        ms.sort((a,b)=>{if(a.pinned&&!b.pinned)return-1;if(!a.pinned&&b.pinned)return 1;return new Date(b.date)-new Date(a.date);});
        return ms;
    }
    getMsgState(id) { const m=this._state.messages.find(x=>x.id===id); return m?getMsgState(m):null; }
}
export const store = new Store();
