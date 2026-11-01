const API_BASE = 'http://127.0.0.1:8080/api';

async function fetchJson(path, opts = {}) {
    const token = localStorage.getItem('admin_token');
    const res = await fetch(`${API_BASE}${path}`, {
        headers: {
            'Content-Type': 'application/json',
            ...(token ? { Authorization: `Bearer ${token}` } : {}),
            ...opts.headers,
        },
        ...opts,
    });
    if (!res.ok) throw new Error(`API error ${res.status}`);
    return res.json();
}

export async function getInstitutions(params = {}) {
    const qs = new URLSearchParams();
    if (params.category) qs.set('category', params.category);
    if (params.diocese_id) qs.set('diocese_id', params.diocese_id);
    if (params.deanery_id) qs.set('deanery_id', params.deanery_id);
    if (params.verified !== undefined) qs.set('verified', params.verified);
    if (params.search) qs.set('search', params.search);
    qs.set('page', params.page || 1);
    qs.set('page_size', params.page_size || 100);
    return fetchJson(`/institutions?${qs}`);
}

export async function getInstitution(id) {
    return fetchJson(`/institutions/${id}`);
}

export async function getDioceses() {
    return fetchJson('/dioceses');
}

export async function searchInstitutions(q) {
    return fetchJson(`/search?q=${encodeURIComponent(q)}`);
}

export async function submitInstitution(data) {
    return fetchJson('/submissions/institution', {
        method: 'POST',
        body: JSON.stringify(data),
    });
}

export async function submitCorrection(data) {
    return fetchJson('/submissions/correction', {
        method: 'POST',
        body: JSON.stringify(data),
    });
}

export async function adminLogin(username, password) {
    return fetchJson('/admin/login', {
        method: 'POST',
        body: JSON.stringify({ username, password }),
    });
}

export async function getAdminSubmissions() {
    return fetchJson('/admin/submissions');
}

export async function approveSubmission(id) {
    return fetchJson(`/admin/submissions/${id}/approve`, { method: 'PUT' });
}

export async function rejectSubmission(id, admin_notes) {
    return fetchJson(`/admin/submissions/${id}/reject`, {
        method: 'PUT',
        body: JSON.stringify({ admin_notes }),
    });
}

export const CATEGORY_COLORS = {
    parish: '#1B4332',
    mission: '#0B3D1E',
    outstation: '#2A9D8F',
    school: '#C9A84C',
    shrine: '#6A1B9A',
    orphanage: '#E76F51',
    office: '#6B7280',
};
