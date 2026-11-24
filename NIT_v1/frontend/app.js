(() => {
  'use strict';

  const API_BASE = window.NIT_API_BASE || `${window.location.origin}/api/v1`;
  const app = document.querySelector('#app');
  const state = { accessToken: null, role: null, email: null };
  const route = new URL(window.location.href);
  const page = route.searchParams.get('page') || 'login';

  function escapeHtml(value) {
    return String(value ?? '').replace(/[&<>"']/g, (character) => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
    })[character]);
  }

  function go(name, params = {}) {
    const next = new URL(window.location.href);
    next.search = new URLSearchParams({ page: name, ...params });
    window.location.assign(next.toString());
  }

  function navLink(pageName, params = {}) {
    const query = new URLSearchParams({ page: pageName, ...params });
    return `?${query.toString()}`;
  }

  function setSession(token, role, email) {
    state.accessToken = token;
    state.role = role;
    state.email = email;
    sessionStorage.setItem('nitAccessToken', token);
    sessionStorage.setItem('nitRole', role);
    sessionStorage.setItem('nitEmail', email);
  }

  function restoreSession() {
    state.accessToken = sessionStorage.getItem('nitAccessToken');
    state.role = sessionStorage.getItem('nitRole');
    state.email = sessionStorage.getItem('nitEmail');
  }

  function clearSession() {
    state.accessToken = null;
    state.role = null;
    state.email = null;
    sessionStorage.removeItem('nitAccessToken');
    sessionStorage.removeItem('nitRole');
    sessionStorage.removeItem('nitEmail');
  }

  async function request(path, options = {}, allowRefresh = true) {
    const headers = new Headers(options.headers || {});
    if (state.accessToken) headers.set('Authorization', `Bearer ${state.accessToken}`);
    if (options.body) headers.set('Content-Type', 'application/json');

    let response;
    try {
      response = await fetch(`${API_BASE}${path}`, {
        ...options,
        headers,
        credentials: 'include'
      });
    } catch (error) {
      throw new Error('Could not reach the API. Check that the backend is running and allows requests from this frontend origin.');
    }

    if (response.status === 401 && allowRefresh && state.accessToken && path !== '/auth/refresh') {
      const refreshed = await fetch(`${API_BASE}/auth/refresh`, {
        method: 'POST',
        credentials: 'include'
      });
      if (refreshed.ok) {
        const data = await refreshed.json();
        state.accessToken = data.data.accessToken;
        sessionStorage.setItem('nitAccessToken', state.accessToken);
        const retryHeaders = new Headers(options.headers || {});
        retryHeaders.set('Authorization', `Bearer ${state.accessToken}`);
        if (options.body) retryHeaders.set('Content-Type', 'application/json');
        response = await fetch(`${API_BASE}${path}`, {
          ...options,
          headers: retryHeaders,
          credentials: 'include'
        });
      } else {
        clearSession();
        go('login', { message: 'Your session expired. Please log in again.' });
        throw new Error('Session expired. Please log in again.');
      }
    }

    if (!response.ok) {
      let message = `Request failed (${response.status})`;
      try {
        const body = await response.json();
        message = body.error?.message || message;
      } catch (_) {}
      throw new Error(message);
    }
    if (response.status === 204) return null;
    return response.json();
  }

  function shell(content, { showLogout = true } = {}) {
    app.innerHTML = `
      <header class="topbar">
        <a class="brand" href="${navLink(state.role === 'admin' ? 'admin' : 'dashboard')}" aria-label="National Index Tracker home">
          <span class="brand-mark">NI</span><span>National Index Tracker</span>
        </a>
        <div class="topbar-actions">
          ${state.email ? `<span class="user-label">${escapeHtml(state.email)}${state.role === 'admin' ? ' · Admin' : ''}</span>` : ''}
          ${showLogout && state.accessToken ? '<button class="button button-secondary" type="button" data-action="logout">Log out</button>' : ''}
        </div>
      </header>
      ${content}
      <footer class="page footer">National Index Tracker <span class="mono">· PUBLIC DATA</span></footer>`;
  }

  function showNotice(form, message, isError = false) {
    const notice = form.querySelector('.notice');
    notice.textContent = message;
    notice.hidden = false;
    notice.classList.toggle('notice-error', isError);
  }

  function authPage(kind) {
    const isRegister = kind === 'register';
    const message = route.searchParams.get('message');
    app.innerHTML = `
      <main class="page-narrow">
        <a class="brand" href="?page=login"><span class="brand-mark">NI</span><span>National Index Tracker</span></a>
        <form class="form-card" data-form="${kind}" style="margin-top:2rem">
          <p class="eyebrow">Public data, clearly presented</p>
          <h1>${isRegister ? 'Create account' : 'Welcome back'}</h1>
          <p class="form-intro">${isRegister ? 'Register to explore sectors and their indices.' : 'Sign in to continue to your dashboard.'}</p>
          <div class="notice" ${message ? '' : 'hidden'}>${escapeHtml(message || '')}</div>
          ${isRegister ? '<div class="field"><label for="name">Name</label><input id="name" name="name" autocomplete="name" required maxlength="100"></div>' : ''}
          <div class="field"><label for="email">Email</label><input id="email" name="email" type="email" autocomplete="email" required></div>
          <div class="field"><label for="password">Password</label><input id="password" name="password" type="password" autocomplete="${isRegister ? 'new-password' : 'current-password'}" minlength="12" maxlength="128" required></div>
          <button class="button" type="submit">${isRegister ? 'Create account' : 'Sign in'}</button>
          <p class="form-footer">${isRegister ? 'Already registered?' : 'New to the tracker?'} <a class="text-link" href="?page=${isRegister ? 'login' : 'register'}">${isRegister ? 'Log in' : 'Create an account'}</a></p>
        </form>
      </main>`;

    app.querySelector('form').addEventListener('submit', async (event) => {
      event.preventDefault();
      const form = event.currentTarget;
      const button = form.querySelector('button[type="submit"]');
      const values = Object.fromEntries(new FormData(form));
      button.disabled = true;
      button.textContent = isRegister ? 'Creating account…' : 'Signing in…';
      try {
        if (isRegister) {
          await request('/auth/register', { method: 'POST', body: JSON.stringify(values) }, false);
          go('login', { message: 'Account created. Please log in.' });
          return;
        }
        const result = await request('/auth/login', { method: 'POST', body: JSON.stringify(values) }, false);
        const data = result.data;
        setSession(data.accessToken, data.role, values.email);
        go(data.role === 'admin' ? 'admin' : 'dashboard');
      } catch (error) {
        showNotice(form, error.message, true);
        button.disabled = false;
        button.textContent = isRegister ? 'Create account' : 'Sign in';
      }
    });
  }

  async function dashboard(isAdmin) {
    if (!requireSession(isAdmin)) return;
    shell('<main class="page" id="content"><p class="eyebrow">Overview</p><h1>Choose a sector</h1><p class="lede">Explore public indicators grouped by sector.</p><div class="empty-state">Loading sectors…</div></main>');
    try {
      const sectors = await request('/interests');
      const content = app.querySelector('#content');
      content.innerHTML = `
        <p class="eyebrow">${isAdmin ? 'Administration' : 'Overview'}</p>
        <h1>${isAdmin ? 'Sector dashboard' : 'Choose a sector'}</h1>
        <p class="lede">Browse public indicators grouped by sector.</p>
        <div class="section-heading"><h2>All sectors</h2><span class="count">${sectors.length} ${sectors.length === 1 ? 'SECTOR' : 'SECTORS'}</span></div>
        ${sectors.length ? `<div class="card-grid">${sectors.map((sector) => `
          <a class="card-link" href="${navLink('sector', { id: sector.id })}">
            <div><h3 class="card-title">${escapeHtml(sector.name)}</h3><p class="card-description">${escapeHtml(sector.description || 'Explore indices in this sector.')}</p></div>
            <span class="card-arrow" aria-hidden="true">↗</span>
          </a>`).join('')}</div>` : '<div class="empty-state">No sectors are available yet.</div>'}
        ${isAdmin ? adminPanel(sectors) : ''}`;
      if (isAdmin) bindAdminForms();
    } catch (error) {
      showPageError(error.message);
    }
  }

  function adminPanel(sectors) {
    return `<section class="admin-panel">
      <p class="eyebrow">Catalog management</p><h2>Manage catalog</h2>
      <div class="admin-forms">
        <form class="admin-form" data-form="create-sector"><h3>Create a sector</h3>
          <div class="field"><label for="sector-name">Name</label><input id="sector-name" name="name" maxlength="100" required></div>
          <div class="field"><label for="sector-description">Description</label><textarea id="sector-description" name="description"></textarea></div>
          <div class="notice" hidden></div><button class="button" type="submit">Add sector</button>
        </form>
        <form class="admin-form" data-form="create-index"><h3>Create an index</h3>
          <div class="field"><label for="index-sector">Sector</label><select id="index-sector" name="interest_id" required>${sectors.map((sector) => `<option value="${sector.id}">${escapeHtml(sector.name)}</option>`).join('')}</select></div>
          <div class="field"><label for="index-name">Name</label><input id="index-name" name="name" required></div>
          <div class="field"><label for="index-description">Description</label><textarea id="index-description" name="description"></textarea></div>
          <div class="notice" hidden></div><button class="button" type="submit" ${sectors.length ? '' : 'disabled'}>Add index</button>
        </form>
      </div>
    </section>`;
  }

  function bindAdminForms() {
    app.querySelector('[data-form="create-sector"]')?.addEventListener('submit', async (event) => {
      event.preventDefault();
      const form = event.currentTarget;
      const data = Object.fromEntries(new FormData(form));
      if (!data.description) data.description = null;
      try {
        await request('/interests', { method: 'POST', body: JSON.stringify(data) });
        go('admin', { message: 'Sector created.' });
      } catch (error) { showNotice(form, error.message, true); }
    });
    app.querySelector('[data-form="create-index"]')?.addEventListener('submit', async (event) => {
      event.preventDefault();
      const form = event.currentTarget;
      const data = Object.fromEntries(new FormData(form));
      data.interest_id = Number(data.interest_id);
      if (!data.description) data.description = null;
      try {
        await request('/indices', { method: 'POST', body: JSON.stringify(data) });
        go('admin', { message: 'Index created.' });
      } catch (error) { showNotice(form, error.message, true); }
    });
  }

  async function sectorPage() {
    if (!requireSession()) return;
    const id = route.searchParams.get('id');
    if (!id) return showPageError('This sector link is missing its ID.');
    shell('<main class="page" id="content"><div class="empty-state">Loading sector…</div></main>');
    try {
      const [sector, indices] = await Promise.all([
        request(`/interests/${encodeURIComponent(id)}`),
        request(`/interests/${encodeURIComponent(id)}/indices`)
      ]);
      app.querySelector('#content').innerHTML = `
        <a class="back-link" href="${navLink(state.role === 'admin' ? 'admin' : 'dashboard')}">← All sectors</a>
        <p class="eyebrow">Sector</p><h1>${escapeHtml(sector.name)}</h1>
        <p class="lede">${escapeHtml(sector.description || 'Indices and public data in this sector.')}</p>
        <div class="section-heading"><h2>Indices</h2><span class="count">${indices.length} ${indices.length === 1 ? 'INDEX' : 'INDICES'}</span></div>
        ${indices.length ? `<div class="index-list">${indices.map((index) => `
          <div class="index-row">
            <a class="index-main" href="${navLink('index', { id: index.id })}"><h3 class="row-title">${escapeHtml(index.name)}</h3><p class="row-description">${escapeHtml(index.description || 'Open the index data.')}</p></a>
            ${state.role === 'admin' ? `<button class="button button-quiet" type="button" data-delete-index="${index.id}" aria-label="Delete ${escapeHtml(index.name)}">Delete</button>` : ''}
          </div>`).join('')}</div>` : '<div class="empty-state">No indices are available in this sector yet.</div>'}
        ${state.role === 'admin' ? `<div class="section-heading"><button class="button button-secondary" type="button" data-delete-sector="${sector.id}">Delete sector</button></div>` : ''}`;
      app.querySelectorAll('[data-delete-index]').forEach((button) => button.addEventListener('click', () => deleteItem(`/indices/${button.dataset.deleteIndex}`, 'index')));
      app.querySelector('[data-delete-sector]')?.addEventListener('click', () => deleteItem(`/interests/${sector.id}`, 'sector', 'dashboard'));
    } catch (error) { showPageError(error.message); }
  }

  async function indexPage() {
    if (!requireSession()) return;
    const id = route.searchParams.get('id');
    if (!id) return showPageError('This index link is missing its ID.');
    shell('<main class="page" id="content"><div class="empty-state">Loading index data…</div></main>');
    try {
      const [index, payload] = await Promise.all([
        request(`/indices/${encodeURIComponent(id)}`),
        request(`/indices/${encodeURIComponent(id)}/payload`)
      ]);
      const sector = await request(`/interests/${index.interest_id}`);
      const fields = Array.isArray(payload.fields) ? payload.fields : [];
      const records = Array.isArray(payload.records) ? payload.records : [];
      const columns = fields.length ? fields.map((field) => ({ key: field.id, label: field.name || field.id })) : [...new Set(records.flatMap((row) => Object.keys(row || {})))].map((key) => ({ key, label: key }));
      const description = payload.description || index.description;
      app.querySelector('#content').innerHTML = `
        <a class="back-link" href="${navLink('sector', { id: sector.id })}">← ${escapeHtml(sector.name)}</a>
        <p class="eyebrow">${escapeHtml(sector.name)}</p><h1>${escapeHtml(index.name)}</h1>
        ${description ? `<p class="lede">${escapeHtml(description)}</p>` : '<p class="lede">Source data for this index.</p>'}
        <div class="data-toolbar"><div><p class="eyebrow">Latest snapshot</p><span class="count">${records.length} RECORDS · ${columns.length} FIELDS</span></div><span class="count">${escapeHtml(index.name)}</span></div>
        ${records.length && columns.length ? `<div class="table-wrap"><table><thead><tr>${columns.map((column) => `<th>${escapeHtml(column.label)}</th>`).join('')}</tr></thead><tbody>${records.map((record) => `<tr>${columns.map((column) => `<td>${formatCell(record?.[column.key])}</td>`).join('')}</tr>`).join('')}</tbody></table></div>` : '<div class="empty-state">This index does not have a saved data snapshot yet.</div>'}
        ${state.role === 'admin' ? '<div class="section-heading"><button class="button button-secondary" type="button" data-delete-index>Delete index</button></div>' : ''}`;
      app.querySelector('[data-delete-index]')?.addEventListener('click', () => deleteItem(`/indices/${id}`, 'index', 'sector', { id: sector.id }));
    } catch (error) { showPageError(error.message); }
  }

  function formatCell(value) {
    if (value === null || value === undefined || value === '') return '<span class="count">—</span>';
    if (typeof value === 'object') return `<span class="mono">${escapeHtml(JSON.stringify(value))}</span>`;
    return escapeHtml(value);
  }

  async function deleteItem(path, label, returnPage = 'sector', params = { id: route.searchParams.get('id') }) {
    if (!window.confirm(`Delete this ${label}?`)) return;
    try {
      await request(path, { method: 'DELETE' });
      go(returnPage, params || {});
    } catch (error) { window.alert(error.message); }
  }

  function requireSession(adminOnly = false) {
    if (!state.accessToken) {
      go('login', { message: 'Please log in to continue.' });
      return false;
    }
    if (adminOnly && state.role !== 'admin') {
      go('dashboard');
      return false;
    }
    return true;
  }

  function showPageError(message) {
    shell(`<main class="page error-page"><p class="eyebrow">Something went wrong</p><h1>We could not load this page.</h1><p>${escapeHtml(message)}</p><a class="button button-secondary" href="${navLink(state.role === 'admin' ? 'admin' : 'dashboard')}">Back to dashboard</a></main>`);
  }

  async function logout() {
    try {
      await request('/auth/logout', { method: 'POST' }, false);
    } catch (_) {
      // Clear local state even if the session has already expired.
    }
    clearSession();
    go('signed-out');
  }

  function signedOut() {
    clearSession();
    shell(`<main class="page error-page"><p class="eyebrow">Session ended</p><h1>You are logged out.</h1><p>Your session has been closed. Sign in again whenever you are ready.</p><a class="button" href="?page=login">Go to login</a></main>`, { showLogout: false });
  }

  document.addEventListener('click', (event) => {
    if (event.target.closest('[data-action="logout"]')) logout();
  });

  restoreSession();
  const message = route.searchParams.get('message');
  if (message && !['register', 'login'].includes(page)) {
    // Dashboard notices are rendered with its content rather than through global markup.
  }
  switch (page) {
    case 'register': authPage('register'); break;
    case 'login': authPage('login'); break;
    case 'dashboard': dashboard(false); break;
    case 'admin': dashboard(true); break;
    case 'sector': sectorPage(); break;
    case 'index': indexPage(); break;
    case 'signed-out': signedOut(); break;
    default: go(state.accessToken ? (state.role === 'admin' ? 'admin' : 'dashboard') : 'login');
  }
})();
