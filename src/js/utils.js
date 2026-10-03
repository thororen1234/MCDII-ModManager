export const $ = (id) => document.getElementById(id);

export const formatDate = (ts) => {
  if (!ts) return 'Unknown';
  return new Date(ts * 1000).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
};

export const formatSize = (bytes) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

export function showToast(msg, type = 'info') {
  const container = $('toast-container');
  if (!container) return;
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.innerText = msg;
  container.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 200);
  }, 3000);
}

export function setStatus(text, error = false) {
  const el = $('status-bar');
  if (!el) return;

  if (!text || text.includes("System ready")) {
    el.classList.remove('active');
    return;
  }

  el.innerText = text;
  el.classList.toggle('error', error);
  el.classList.add('active');
}

export function showModal(id) {
  const el = $(id);
  if (el) el.classList.add('open');
  else console.error(`Modal with ID ${id} not found.`);
}

export function closeModal(id) {
  const el = $(id);
  if (el) el.classList.remove('open');
}

export async function ask(message, options = {}) {
  const title = options.title || 'Confirm';
  const yesText = options.okLabel || 'Yes';
  const noText = options.cancelLabel || 'No';

  const modal = $('modal-confirm');
  const titleEl = $('modal-confirm-title');
  const bodyEl = $('modal-confirm-body');
  const yesBtn = $('modal-confirm-yes');
  const noBtn = $('modal-confirm-no');

  if (!modal || !titleEl || !bodyEl || !yesBtn || !noBtn) {
    console.error('Confirm modal elements missing');
    return window.confirm(message);
  }

  titleEl.innerText = title;
  bodyEl.innerHTML = message;
  yesBtn.innerText = yesText;
  noBtn.innerText = noText;

  modal.classList.remove('kind-warning', 'kind-error', 'kind-info');
  if (options.kind) {
    modal.classList.add(`kind-${options.kind}`);
  }

  showModal('modal-confirm');

  return new Promise((resolve) => {
    const onYes = () => {
      cleanup();
      resolve(true);
    };
    const onNo = () => {
      cleanup();
      resolve(false);
    };
    const cleanup = () => {
      yesBtn.removeEventListener('click', onYes);
      noBtn.removeEventListener('click', onNo);
      closeModal('modal-confirm');
    };

    yesBtn.addEventListener('click', onYes);
    noBtn.addEventListener('click', onNo);
  });
}

export async function message(msg, options = {}) {
  const title = options.title || 'Message';
  const okText = options.okLabel || 'OK';

  const modal = $('modal-confirm');
  const titleEl = $('modal-confirm-title');
  const bodyEl = $('modal-confirm-body');
  const yesBtn = $('modal-confirm-yes');
  const noBtn = $('modal-confirm-no');

  if (!modal) return window.alert(msg);

  titleEl.innerText = title;
  bodyEl.innerHTML = msg;
  yesBtn.innerText = okText;
  noBtn.style.display = 'none';

  showModal('modal-confirm');

  return new Promise((resolve) => {
    const onOk = () => {
      noBtn.style.display = 'inline-flex';
      yesBtn.removeEventListener('click', onOk);
      closeModal('modal-confirm');
      resolve();
    };
    yesBtn.addEventListener('click', onOk);
  });
}
