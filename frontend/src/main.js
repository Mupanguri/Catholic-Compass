import {
    getInstitutions,
    getInstitution,
    getDioceses,
    searchInstitutions,
    submitInstitution,
    submitCorrection,
    adminLogin,
    getAdminSubmissions,
    approveSubmission,
    rejectSubmission,
    CATEGORY_COLORS,
} from './utils/api.js';

const app = document.getElementById('app');

let map = null;
let markers = [];
let currentInstitutionId = null;
let allInstitutions = [];
let resolveMapReady;
const mapReady = new Promise((resolve) => {
    resolveMapReady = resolve;
});

function getLiturgicalTheme(date = new Date()) {
    const month = date.getMonth() + 1;
    const day = date.getDate();
    const key = `${date.getFullYear()}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;

    if (key >= `${date.getFullYear()}-02-14` && key <= `${date.getFullYear()}-04-04`) {
        return { name: 'Lent / Triduum', color: '#991b1b' };
    }
    if (key >= `${date.getFullYear()}-04-05` && key <= `${date.getFullYear()}-05-24`) {
        return { name: 'Eastertide', color: '#f8fafc' };
    }
    if (key >= `${date.getFullYear()}-11-29` || key <= `${date.getFullYear()}-01-11`) {
        return { name: 'Advent / Christmas', color: '#5b21b6' };
    }
    return { name: 'Ordinary Time', color: '#1b4332' };
}

function applyLiturgicalTheme() {
    const theme = getLiturgicalTheme();
    document.documentElement.style.setProperty('--liturgical-color', theme.color);
    document.documentElement.dataset.liturgicalSeason = theme.name;
    return theme;
}

applyLiturgicalTheme();

let sidebar;
let institutionList;
let searchInput;
let categoryPills;
let dioceseSelect;
let addPlaceBtn;

function initMap() {
    map = new maplibregl.Map({
        container: 'map',
        style: 'https://basemaps.cartocdn.com/gl/voyager-gl-style/style.json',
        center: [29.1549, -19.0154],
        zoom: 6,
    });

    map.addControl(new maplibregl.NavigationControl());

    map.on('load', () => {
        map.addSource('institutions', {
            type: 'geojson',
            data: { type: 'FeatureCollection', features: [] },
            cluster: true,
            clusterMaxZoom: 14,
            clusterRadius: 50,
        });

        map.addSource('route', {
            type: 'geojson',
            data: {
                type: 'Feature',
                properties: {},
                geometry: { type: 'LineString', coordinates: [] },
            },
        });

        map.addLayer({
            id: 'route-layer',
            type: 'line',
            source: 'route',
            layout: {
                'line-join': 'round',
                'line-cap': 'round',
            },
            paint: {
                'line-color': '#1B4332',
                'line-width': 5,
                'line-opacity': 0.8,
            },
        });

        map.addLayer({
            id: 'clusters',
            type: 'circle',
            source: 'institutions',
            filter: ['has', 'point_count'],
            paint: {
                'circle-color': CATEGORY_COLORS.parish,
                'circle-radius': ['step', ['get', 'point_count'], 20, 100, 30, 150, 40],
            },
        });

        map.addLayer({
            id: 'cluster-count',
            type: 'symbol',
            source: 'institutions',
            filter: ['has', 'point_count'],
            layout: {
                'text-field': '{point_count}',
                'text-font': ['Open Sans Regular'],
                'text-size': 12,
            },
        });

        map.addLayer({
            id: 'unclustered-point',
            type: 'circle',
            source: 'institutions',
            filter: ['!', ['has', 'point_count']],
            paint: {
                'circle-color': [
                    'match',
                    ['get', 'category'],
                    'parish', CATEGORY_COLORS.parish,
                    'mission', CATEGORY_COLORS.mission,
                    'outstation', CATEGORY_COLORS.outstation,
                    'school', CATEGORY_COLORS.school,
                    'shrine', CATEGORY_COLORS.shrine,
                    'orphanage', CATEGORY_COLORS.orphanage,
                    'office', CATEGORY_COLORS.office,
                    '#6B7280',
                ],
                'circle-radius': 7,
                'circle-stroke-color': '#fff',
                'circle-stroke-width': 2,
            },
        });

        map.on('click', 'clusters', (e) => {
            const features = map.queryRenderedFeatures(e.point, { layers: ['clusters'] });
            const clusterId = features[0].properties.cluster_id;
            map.getSource('institutions').getClusterExpansionZoom(clusterId).then((zoom) => {
                map.easeTo({ center: features[0].geometry.coordinates, zoom });
            });
        });

        map.on('click', 'unclustered-point', (e) => {
            const props = e.features[0].properties;
            openDetail(props.id);
        });

        map.on('mouseenter', 'unclustered-point', () => {
            map.getCanvas().style.cursor = 'pointer';
        });

        map.on('mouseleave', 'unclustered-point', () => {
            map.getCanvas().style.cursor = '';
        });

        map.on('click', (e) => {
            const features = map.queryRenderedFeatures(e.point, {
                layers: ['clusters', 'unclustered-point'],
            });
            if (features.length === 0) {
                closeDetail();
            }
        });

        resolveMapReady();
    });
}

function renderMarkers(institutions) {
    const source = map?.getSource('institutions');
    if (!source) return;

    const features = institutions.map((inst) => {
        if (!inst.location) return null;
        const coordinates = parsePointLocation(inst.location);
        if (!coordinates) return null;
        return {
            type: 'Feature',
            geometry: { type: 'Point', coordinates },
            properties: {
                id: inst.id,
                name: inst.name,
                category: inst.category,
            },
        };
    }).filter(Boolean);

    source.setData({
        type: 'FeatureCollection',
        features,
    });
}

function parsePointLocation(location) {
    const match = location.match(/POINT(?:\s*Z)?\s*\(\s*([-\d.]+)\s+([-\d.]+)\s*\)/i);
    if (!match) return null;

    const lng = Number(match[1]);
    const lat = Number(match[2]);
    if (!Number.isFinite(lng) || !Number.isFinite(lat)
        || lng < -180 || lng > 180 || lat < -90 || lat > 90) {
        return null;
    }
    return [lng, lat];
}

function focusOnInstitution(coords) {
    if (!map || !coords) return;
    map.flyTo({
        center: coords,
        zoom: 15,
        essential: true,
    });
}

function renderInstitutionList(institutions) {
    institutionList.innerHTML = '';
    for (const inst of institutions) {
        const card = document.createElement('div');
        card.className = 'institution-card';
        card.innerHTML = `
            <div class="card-header">
                <div class="card-name">${escapeHtml(inst.name)}</div>
                <span class="category-badge" style="background:${categoryColor(inst.category)}">${inst.category}</span>
            </div>
            <div class="card-meta">
                ${inst.address_notes ? `<span>${escapeHtml(inst.address_notes)}</span>` : ''}
            </div>
            <div class="card-footer">
                ${inst.verified ? '<span class="verified-badge">Verified</span>' : '<span class="unverified-badge">Unverified</span>'}
            </div>
        `;
        card.onclick = (e) => {
            e.stopPropagation();
            const coords = parsePointLocation(inst.location);
            focusOnInstitution(coords);
            openDetail(inst.id);
        };
        institutionList.appendChild(card);
    }
}

function categoryColor(cat) {
    return CATEGORY_COLORS[cat] || '#6B7280';
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text || '';
    return div.innerHTML;
}

async function openDetail(id) {
    if (currentInstitutionId === id) {
        const institution = allInstitutions.find((item) => item.id === id);
        focusOnInstitution(parsePointLocation(institution?.location));
        return;
    }
    currentInstitutionId = id;

    try {
        const data = await getInstitution(id);
        const inst = data.institution;
        const images = data.images || [];

        const coords = parsePointLocation(inst.location);

        if (coords) {
            focusOnInstitution(coords);
            if (map.getLayer('unclustered-point')) {
                map.setFilter('unclustered-point', ['==', ['get', 'id'], id]);
            }
        }

        const detailPanel = document.getElementById('detail-panel') || createDetailPanel();
        detailPanel.innerHTML = detailHtml(inst, images, coords);
        detailPanel.classList.add('open');
        setupDetailActions(detailPanel, coords);
        clearRoute();

        if (coords) {
            new maplibregl.Marker({ color: categoryColor(inst.category) })
                .setLngLat(coords)
                .setPopup(new maplibregl.Popup().setHTML(`<strong>${escapeHtml(inst.name)}</strong>`))
                .addTo(map);
        }
    } catch (err) {
        console.error(err);
    }
}

function createDetailPanel() {
    const panel = document.createElement('div');
    panel.id = 'detail-panel';
    panel.className = 'detail-panel';
    panel.innerHTML = `<button class="detail-close">&times;</button><div class="detail-content"></div>`;
    panel.querySelector('.detail-close').onclick = closeDetail;
    app.appendChild(panel);
    return panel;
}

function closeDetail() {
    currentInstitutionId = null;
    const panel = document.getElementById('detail-panel');
    if (panel) panel.classList.remove('open');
    if (map.isStyleLoaded()) {
        map.setFilter('unclustered-point', ['!', ['has', 'point_count']]);
    }
}

function detailHtml(inst, images, coords) {
    const coverUrl = inst.cover_image_url || `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><rect width="200" height="200" fill="${categoryColor(inst.category)}"/><text x="50%" y="50%" font-size="14" fill="white" text-anchor="middle">${escapeHtml(inst.category)}</text></svg>`;

    const galleryHtml = images.length > 0
        ? images.map(img => `<img src="${img.url}" class="gallery-thumb" onclick="openImage('${img.url}', '${escapeHtml(img.caption||'')}')" alt="${escapeHtml(img.caption||'')}">`).join('')
        : '<p style="text-align:center;color:#666">No photos yet — be the first to submit one</p><button onclick="openImageUpload(\'' + inst.id + '\')">Submit a photo</button>';

    return `
        <div class="detail-header">
            <h2>${escapeHtml(inst.name)}</h2>
            <span class="category-badge" style="background:${categoryColor(inst.category)}">${inst.category}</span>
            ${inst.verified ? '<span class="verified-badge">✓ Verified</span>' : '<span class="unverified-badge">Unverified</span>'}
        </div>
        <div class="detail-hero">
            <img src="${coverUrl}" alt="${escapeHtml(inst.name)}">
        </div>
        <div class="detail-gallery">
            <h3>Gallery</h3>
            <div class="gallery-strip">${galleryHtml}</div>
        </div>
        <div class="detail-info">
            ${inst.priest_in_charge ? `<p><strong>Priest:</strong> ${escapeHtml(inst.priest_in_charge)}</p>` : ''}
            ${inst.contact_phone ? `<p><strong>Phone:</strong> <a href="tel:${inst.contact_phone}">${escapeHtml(inst.contact_phone)}</a></p>` : ''}
            ${inst.contact_email ? `<p><strong>Email:</strong> <a href="mailto:${inst.contact_email}">${escapeHtml(inst.contact_email)}</a></p>` : ''}
            ${inst.website ? `<p><strong>Website:</strong> <a href="${inst.website}" target="_blank">${escapeHtml(inst.website)}</a></p>` : ''}
            ${inst.address_notes ? `<p><strong>Directions:</strong> ${escapeHtml(inst.address_notes)}</p>` : ''}
            ${coords ? `
                <p><strong>GPS:</strong> ${coords[1].toFixed(4)}, ${coords[0].toFixed(4)}</p>
                <div class="navigation-actions">
                    <label>
                        Travel mode
                        <select class="travel-mode">
                            <option value="driving">Driving</option>
                            <option value="walking">Walking</option>
                            <option value="transit">Public transit</option>
                        </select>
                    </label>
                    <div class="navigation-buttons">
                        <button type="button" class="nav-btn route-map-btn">Route on Map</button>
                        <button type="button" class="nav-btn get-directions-btn">Open Maps</button>
                        <button type="button" class="nav-btn copy-gps-btn">Copy GPS</button>
                    </div>
                    <span class="navigation-status" role="status"></span>
                </div>
            ` : ''}
        </div>
        <div class="detail-actions">
            <button onclick="openCorrectionForm('${inst.id}')">Submit a correction</button>
        </div>
    `;
}

function setupDetailActions(panel, coords) {
    if (!coords) return;

    const directionsButton = panel.querySelector('.get-directions-btn');
    const routeMapButton = panel.querySelector('.route-map-btn');
    const copyButton = panel.querySelector('.copy-gps-btn');
    const travelMode = panel.querySelector('.travel-mode');
    const status = panel.querySelector('.navigation-status');
    const [lng, lat] = coords;

    directionsButton.addEventListener('click', () => {
        handleGetDirections(lat, lng, travelMode.value, directionsButton, status);
    });

    routeMapButton.addEventListener('click', () => {
        drawRouteOnMap(lat, lng, routeMapButton, status);
    });

    copyButton.addEventListener('click', async () => {
        const coordinates = `${lat.toFixed(6)}, ${lng.toFixed(6)}`;
        try {
            await navigator.clipboard.writeText(coordinates);
            status.textContent = 'Coordinates copied.';
        } catch (err) {
            status.textContent = 'Copy failed. Select the coordinates above manually.';
            console.error('Unable to copy coordinates:', err);
        }
    });
}

function openImage(url, caption) {
    const d = document.createElement('div');
    d.className = 'image-modal';
    d.innerHTML = `<div class="image-modal-content"><button class="image-modal-close">&times;</button><img src="${url}" style="max-width:90vw;max-height:80vh"><p>${escapeHtml(caption)}</p></div>`;
    d.querySelector('.image-modal-close').onclick = () => d.remove();
    d.onclick = (e) => { if (e.target === d) d.remove(); };
    document.body.appendChild(d);
}

function openImageUpload(institutionId) {
    alert('Image uploads are temporarily disabled during testing.');
}

function handleGetDirections(lat, lng, travelMode, button, status) {
    const fallbackUrl = `https://www.google.com/maps/search/?api=1&query=${lat},${lng}`;
    const openRoute = (origin) => {
        const originParam = origin ? `&origin=${origin[0]},${origin[1]}` : '';
        const url = `https://www.google.com/maps/dir/?api=1${originParam}&destination=${lat},${lng}&travelmode=${travelMode}`;
        window.open(url, '_blank', 'noopener');
    };

    button.disabled = true;
    button.textContent = 'Locating...';
    status.textContent = 'Requesting your location...';

    if (!navigator.geolocation) {
        status.textContent = 'Location is unavailable; opening the destination.';
        window.open(fallbackUrl, '_blank', 'noopener');
        button.disabled = false;
        button.textContent = 'Get Directions';
        return;
    }

    navigator.geolocation.getCurrentPosition(
        (position) => {
            openRoute([position.coords.latitude, position.coords.longitude]);
            button.disabled = false;
            button.textContent = 'Get Directions';
            status.textContent = '';
        },
        (error) => {
            console.warn('Geolocation unavailable:', error.message);
            status.textContent = 'Location unavailable; opening the destination.';
            window.open(fallbackUrl, '_blank', 'noopener');
            button.disabled = false;
            button.textContent = 'Get Directions';
        },
        { enableHighAccuracy: true, timeout: 10000, maximumAge: 0 },
    );
}

function clearRoute() {
    const source = map?.getSource('route');
    if (source) {
        source.setData({
            type: 'Feature',
            properties: {},
            geometry: { type: 'LineString', coordinates: [] },
        });
    }
}

function drawRouteOnMap(destLat, destLng, button, status) {
    if (!navigator.geolocation) {
        status.textContent = 'Your browser does not support location.';
        return;
    }

    button.disabled = true;
    button.textContent = 'Locating...';
    status.textContent = 'Requesting your location...';

    navigator.geolocation.getCurrentPosition(async (position) => {
        const userLng = position.coords.longitude;
        const userLat = position.coords.latitude;
        const url = `https://router.project-osrm.org/route/v1/driving/${userLng},${userLat};${destLng},${destLat}?overview=full&geometries=geojson`;

        try {
            const response = await fetch(url);
            if (!response.ok) throw new Error(`Routing service returned ${response.status}`);
            const data = await response.json();
            if (data.code !== 'Ok' || !data.routes?.length) {
                throw new Error('No drivable route was found');
            }

            const route = data.routes[0];
            map.getSource('route').setData({
                type: 'Feature',
                properties: {},
                geometry: route.geometry,
            });

            const bounds = route.geometry.coordinates.reduce(
                (result, coordinate) => result.extend(coordinate),
                new maplibregl.LngLatBounds(route.geometry.coordinates[0], route.geometry.coordinates[0]),
            );
            map.fitBounds(bounds, { padding: 60, maxZoom: 14 });

            const distanceKm = (route.distance / 1000).toFixed(1);
            const durationMinutes = Math.round(route.duration / 60);
            status.textContent = `${distanceKm} km · approximately ${durationMinutes} min by road`;
        } catch (err) {
            console.error('Unable to load route:', err);
            status.textContent = 'Could not calculate a road route. Try Open Maps instead.';
        } finally {
            button.disabled = false;
            button.textContent = 'Route on Map';
        }
    }, (error) => {
        console.warn('Geolocation unavailable:', error.message);
        status.textContent = 'Location unavailable. Allow location access to draw a route.';
        button.disabled = false;
        button.textContent = 'Route on Map';
    }, { enableHighAccuracy: true, timeout: 10000, maximumAge: 0 });
}

function openCorrectionForm(id) {
    const field = prompt('Which field needs correction? (name, address_notes, contact_phone, contact_email, website, priest_in_charge)');
    if (!field) return;
    const value = prompt('What is the correct value?');
    if (!value) return;
    const notes = prompt('Additional notes (optional):') || undefined;
    submitCorrection({
        institution_id: id,
        correction_field: field,
        correction_value: value,
        additional_notes: notes,
    }).then(() => alert('Thank you — your correction will be reviewed.'))
        .catch((err) => alert(`Correction failed: ${err.message}`));
}

async function renderAdmin() {
    const theme = applyLiturgicalTheme();
    app.innerHTML = `
        <main class="admin-page">
            <div class="admin-shell">
                <aside class="admin-sidebar">
                    <div class="admin-brand">
                        <img src="/assets/compass.svg" alt="" class="admin-logo">
                        <strong>Catholic Compass</strong>
                    </div>
                    <p class="admin-sidebar-label">Administration</p>
                    <nav class="admin-nav">
                        <a class="active" href="/admin">Review queue</a>
                        <a href="/">Public map</a>
                    </nav>
                    <div class="admin-season">
                        <span>Liturgical season</span>
                        <strong>${theme.name}</strong>
                    </div>
                    <footer class="admin-sidebar-footer">Akroatis copyright 2026</footer>
                </aside>
                <section class="admin-workspace">
                    <div id="admin-content"></div>
                </section>
            </div>
        </main>
    `;
    const content = document.getElementById('admin-content');
    const token = localStorage.getItem('admin_token');
    if (!token) {
        content.innerHTML = `
            <h1>Sign in</h1>
            <p class="admin-help">Review parishioner submissions and corrections.</p>
            <form id="admin-login-form" class="admin-form">
                <input name="username" placeholder="Username" required autocomplete="username">
                <input name="password" type="password" placeholder="Password" required autocomplete="current-password">
                <button class="landing-cta" type="submit">Sign in</button>
                <p class="admin-error" id="admin-error"></p>
            </form>
        `;
        document.getElementById('admin-login-form').addEventListener('submit', async (event) => {
            event.preventDefault();
            const form = new FormData(event.currentTarget);
            try {
                const result = await adminLogin(form.get('username'), form.get('password'));
                localStorage.setItem('admin_token', result.token);
                renderAdmin();
            } catch (error) {
                document.getElementById('admin-error').textContent = 'Invalid credentials.';
            }
        });
        return;
    }

    content.innerHTML = `
        <header class="admin-header">
            <div><p class="eyebrow">Command center</p><h1>Review queue</h1><p class="admin-help">Keep the Catholic Compass accurate and welcoming for every pilgrim.</p></div>
            <button id="admin-logout" class="admin-link">Sign out</button>
        </header>
        <div class="admin-metrics">
            <div class="metric-card"><span>Total institutions</span><strong id="metric-total">—</strong></div>
            <div class="metric-card"><span>Pending reviews</span><strong id="metric-pending">—</strong></div>
            <div class="metric-card"><span>Verified institutions</span><strong id="metric-verified">—</strong></div>
        </div>
        <div class="admin-queue-header"><h2>Parishioner submissions</h2><span class="theme-chip">${theme.name}</span></div>
        <div id="submission-queue"><p class="admin-help">Loading pending reviews...</p></div>
        <footer class="admin-footer">Akroatis copyright 2026</footer>
    `;
    document.getElementById('admin-logout').onclick = () => {
        localStorage.removeItem('admin_token');
        renderAdmin();
    };
    try {
        const submissions = await getAdminSubmissions();
        const institutionData = await getInstitutions({ page: 1, page_size: 100 });
        document.getElementById('metric-total').textContent = institutionData.total;
        document.getElementById('metric-pending').textContent = submissions.filter((submission) => submission.status === 'pending').length;
        document.getElementById('metric-verified').textContent = institutionData.institutions.filter((institution) => institution.verified).length;
        const queue = document.getElementById('submission-queue');
        const pending = submissions.filter((submission) => submission.status === 'pending');
        queue.innerHTML = pending.length ? pending.map((submission) => `
            <article class="submission-card">
                <div>
                    <span class="submission-type">${submission.submission_type}</span>
                    <h2>${escapeHtml(submission.proposed_name || `Correction for ${submission.institution_id || 'institution'}`)}</h2>
                    <p>${escapeHtml(submission.proposed_category || submission.correction_value || '')}</p>
                    <small>Submitted by ${escapeHtml(submission.submitter_name || 'Anonymous')}</small>
                </div>
                <div class="submission-actions">
                    <button class="approve-btn" data-id="${submission.id}">Approve</button>
                    <button class="reject-btn" data-id="${submission.id}">Reject</button>
                </div>
            </article>
        `).join('') : '<p>No pending submissions.</p>';
        queue.querySelectorAll('.approve-btn').forEach((button) => {
            button.onclick = async () => {
                await approveSubmission(button.dataset.id);
                renderAdmin();
            };
        });
        queue.querySelectorAll('.reject-btn').forEach((button) => {
            button.onclick = async () => {
                const reason = prompt('Reason for rejection:');
                if (!reason) return;
                await rejectSubmission(button.dataset.id, reason);
                renderAdmin();
            };
        });
    } catch (error) {
        content.innerHTML += `<p class="admin-error">Session expired or access denied. Please sign in again.</p>`;
        localStorage.removeItem('admin_token');
    }
}

async function loadDioceses() {
    try {
        const dioceses = await getDioceses();
        dioceseSelect.innerHTML = '<option value="">All Dioceses</option>';
        for (const d of dioceses) {
            const opt = document.createElement('option');
            opt.value = d.id;
            opt.textContent = d.name;
            dioceseSelect.appendChild(opt);
        }
    } catch (err) {
        console.error(err);
    }
}

async function loadInstitutions() {
    try {
        await mapReady;
        const categoryPill = document.querySelector('.pill.active');
        const category = categoryPill ? categoryPill.dataset.category : 'all';
        const diocese_id = dioceseSelect.value;

        const params = {};
        if (category !== 'all') params.category = category;
        if (diocese_id) params.diocese_id = diocese_id;

        const data = await getInstitutions(params);
        allInstitutions = data.institutions;
        renderInstitutionList(allInstitutions);
        renderMarkers(allInstitutions);
    } catch (err) {
        console.error(err);
    }
}

async function handleSearch(q) {
    if (!q) {
        loadInstitutions();
        return;
    }
    try {
        await mapReady;
        const results = await searchInstitutions(q);
        allInstitutions = results;
        renderInstitutionList(allInstitutions);
        renderMarkers(allInstitutions);
    } catch (err) {
        console.error(err);
    }
}

function setupEventListeners() {
    document.getElementById('map-cta').addEventListener('click', () => {
        document.querySelector('.landing-page').hidden = true;
        document.querySelector('.map-view').hidden = false;
        map.resize();
    });

    searchInput.addEventListener('input', (e) => {
        handleSearch(e.target.value);
    });

    categoryPills.forEach((pill) => {
        pill.addEventListener('click', () => {
            categoryPills.forEach(p => p.classList.remove('active'));
            pill.classList.add('active');
            loadInstitutions();
        });
    });

    dioceseSelect.addEventListener('change', loadInstitutions);

    const addPlaceModal = document.getElementById('add-place-modal');
    const addPlaceForm = document.getElementById('add-place-form');
    const closeAddPlaceModal = () => {
        addPlaceModal.hidden = true;
        addPlaceForm.reset();
        document.getElementById('add-place-status').textContent = '';
    };

    addPlaceBtn.addEventListener('click', () => {
        addPlaceModal.hidden = false;
        document.getElementById('new-name').focus();
    });
    document.querySelectorAll('[data-close-add-place]').forEach((button) => {
        button.addEventListener('click', closeAddPlaceModal);
    });
    addPlaceModal.addEventListener('click', (event) => {
        if (event.target === addPlaceModal) closeAddPlaceModal();
    });
    addPlaceForm.addEventListener('submit', async (event) => {
        event.preventDefault();
        const submitButton = document.getElementById('add-place-submit');
        const status = document.getElementById('add-place-status');
        const form = new FormData(addPlaceForm);
        const lat = form.get('proposed_lat');
        const lng = form.get('proposed_lng');
        const payload = {
            proposed_name: form.get('proposed_name').trim(),
            proposed_category: form.get('proposed_category'),
            proposed_lat: lat ? Number(lat) : undefined,
            proposed_lng: lng ? Number(lng) : undefined,
            proposed_address_notes: form.get('proposed_address_notes').trim(),
            submitter_name: form.get('submitter_name').trim() || undefined,
            submitter_email: form.get('submitter_email').trim() || undefined,
        };

        submitButton.disabled = true;
        submitButton.textContent = 'Submitting...';
        status.textContent = '';
        try {
            await submitInstitution(payload);
            status.className = 'modal-status success';
            status.textContent = 'Thank you. Your place will appear after admin review.';
            addPlaceForm.reset();
            setTimeout(closeAddPlaceModal, 1800);
        } catch (error) {
            status.className = 'modal-status error';
            status.textContent = `Submission failed: ${error.message}`;
        } finally {
            submitButton.disabled = false;
            submitButton.textContent = 'Submit place';
        }
    });

    document.addEventListener('click', (e) => {
        const panel = document.getElementById('detail-panel');
        if (panel && panel.classList.contains('open') && !panel.contains(e.target) && e.target.closest('.institution-card') === null) {
            closeDetail();
        }
    });
}

window.openImage = openImage;
window.openImageUpload = openImageUpload;
window.openCorrectionForm = openCorrectionForm;

app.innerHTML = `
    <section class="landing-page">
        <nav class="landing-nav">
            <div class="brand-lockup">
                <span class="brand-mark" aria-hidden="true">✦</span>
                <span>Catholic Compass</span>
            </div>
            <span class="brand-subtitle">A guide to places of faith</span>
        </nav>
        <div class="landing-content">
            <div class="landing-copy">
                <p class="eyebrow">Faith finds a way</p>
                <h1>Wherever the journey takes you, <em>faith travels with you.</em></h1>
                <p class="landing-lead">
                    We are a pilgrim Church. When one member moves, the journey does not end
                    and neither does his or her faith. Guided by God’s gifts and the Holy Spirit,
                    we created Catholic Compass to help every traveller find a parish, mission,
                    shrine, school, or community of faith—nearby or far away.
                </p>
                <button id="map-cta" class="landing-cta">Take me to the map <span aria-hidden="true">→</span></button>
            </div>
            <div class="landing-visual" aria-label="Catholic Compass">
                <div class="compass-halo"></div>
                <div class="compass">
                                    <img src="/assets/compass.jpeg" alt="Catholic Compass" class="landing-logo">
                    <span class="compass-cross">✚</span>
                    <span class="compass-north">N</span>
                    <span class="compass-east">E</span>
                    <span class="compass-south">S</span>
                    <span class="compass-west">W</span>
                    <span class="compass-needle"></span>
                </div>
            </div>
        </div>
        <div class="landing-footer">
            <span>Parishes · Missions · Shrines · Schools</span>
                <span>“Be strong in the Lord and in his mighty power.” — Ephesians 6:10 · Akroatis copyright 2026</span>
        </div>
    </section>
    <div class="map-view" hidden>
        <div class="layout">
            <aside class="sidebar">
                <div class="sidebar-header">
                    <h1>Catholic Compass</h1>
                </div>
                <div class="search-section">
                    <input type="text" id="search" placeholder="Search institutions..." class="search-input">
                </div>
                <div class="filters">
                    <div class="category-pills">
                        <button class="pill active" data-category="all">All</button>
                        <button class="pill" data-category="parish">Parish</button>
                        <button class="pill" data-category="mission">Mission</button>
                        <button class="pill" data-category="outstation">Outstation</button>
                        <button class="pill" data-category="school">School</button>
                        <button class="pill" data-category="shrine">Shrine</button>
                        <button class="pill" data-category="orphanage">Orphanage</button>
                    </div>
                    <select id="diocese-filter" class="diocese-select">
                        <option value="">All Dioceses</option>
                    </select>
                </div>
                <div class="institution-list" id="institution-list"></div>
                <button class="add-place-btn">Add a place</button>
            </aside>
            <main class="map-container">
                <div id="map"></div>
            </main>
        </div>
        <div id="add-place-modal" class="modal-backdrop" hidden>
            <section class="form-modal" role="dialog" aria-modal="true" aria-labelledby="add-place-title">
                <button type="button" class="modal-close" data-close-add-place aria-label="Close">&times;</button>
                <p class="eyebrow">Help build the directory</p>
                <h2 id="add-place-title">Add a place</h2>
                <p class="modal-intro">Share a place of faith with fellow pilgrims. An administrator will review it before publication.</p>
                <form id="add-place-form" class="place-form">
                    <label>Institution name<input id="new-name" name="proposed_name" required maxlength="255" placeholder="St. Peter’s Parish"></label>
                    <label>Category
                        <select name="proposed_category" required>
                            <option value="parish">Parish</option>
                            <option value="outstation">Outstation</option>
                            <option value="mission">Mission</option>
                            <option value="school">School</option>
                            <option value="orphanage">Orphanage</option>
                            <option value="shrine">Shrine</option>
                            <option value="office">Office</option>
                        </select>
                    </label>
                    <div class="form-row">
                        <label>Latitude<input name="proposed_lat" type="number" step="any" min="-90" max="90" placeholder="-17.8292"></label>
                        <label>Longitude<input name="proposed_lng" type="number" step="any" min="-180" max="180" placeholder="31.0522"></label>
                    </div>
                    <label>Address or notes<textarea name="proposed_address_notes" rows="3" maxlength="2000" placeholder="Tell us how to find this place"></textarea></label>
                    <div class="form-row">
                        <label>Your name (optional)<input name="submitter_name" maxlength="150"></label>
                        <label>Email (optional)<input name="submitter_email" type="email" maxlength="150"></label>
                    </div>
                    <p id="add-place-status" class="modal-status" role="status"></p>
                    <div class="modal-actions">
                        <button type="button" class="modal-secondary" data-close-add-place>Cancel</button>
                        <button type="submit" class="modal-primary" id="add-place-submit">Submit place</button>
                    </div>
                </form>
            </section>
        </div>
    </div>
`;

if (window.location.pathname.startsWith('/admin')) {
    renderAdmin();
} else {
    sidebar = document.querySelector('.sidebar');
    institutionList = document.getElementById('institution-list');
    searchInput = document.getElementById('search');
    categoryPills = document.querySelectorAll('.pill');
    dioceseSelect = document.getElementById('diocese-filter');
    addPlaceBtn = document.querySelector('.add-place-btn');
    setupEventListeners();
    initMap();
    loadDioceses();
    loadInstitutions();
}
