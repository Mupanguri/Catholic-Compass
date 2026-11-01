const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 5173;
const FRONTEND_DIR = path.join(__dirname);

const MIME_TYPES = {
    '.html': 'text/html',
    '.js': 'text/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpg',
    '.svg': 'image/svg+xml',
};

function serveFile(res, filePath) {
    const ext = path.extname(filePath).toLowerCase();
    const contentType = MIME_TYPES[ext] || 'application/octet-stream';
    fs.readFile(filePath, (err, data) => {
        if (err) {
            res.writeHead(404);
            res.end('Not found');
            return;
        }
        res.writeHead(200, {
            'Content-Type': contentType,
            'Cache-Control': 'no-store',
            'Access-Control-Allow-Origin': '*',
        });
        res.end(data);
    });
}

const server = http.createServer((req, res) => {
    let filePath = path.join(FRONTEND_DIR, req.url === '/' ? 'index.html' : req.url);
    filePath = path.normalize(filePath);
    if (!filePath.startsWith(FRONTEND_DIR)) {
        res.writeHead(403);
        res.end('Forbidden');
        return;
    }
    serveFile(res, filePath);
});

server.listen(PORT, '0.0.0.0', () => {
    console.log(`Frontend server running on http://localhost:${PORT}`);
});
