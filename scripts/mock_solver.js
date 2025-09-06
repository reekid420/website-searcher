// Simple mock FlareSolverr server for local testing
const http = require('http');

const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 8192;

const fitgirlHtml = '<html><h2 class="entry-title"><a href="https://fitgirl-repacks.site/game">Elden Ring Game Page</a></h2></html>';
const csrinHtml = '<html>\n<a class="topictitle" href="viewtopic.php?t=111">Elden Ring One</a>\n<a class="topictitle" href="viewtopic.php?t=222">Elden Ring Two</a>\n</html>';

const server = http.createServer((req, res) => {
  if (req.method === 'POST' && req.url === '/') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      let responseHtml = fitgirlHtml;
      try {
        const json = JSON.parse(body || '{}');
        const target = (json && json.url) || '';
        if (typeof target === 'string' && target.includes('cs.rin.ru')) {
          responseHtml = csrinHtml;
        }
      } catch {}
      const payload = JSON.stringify({ solution: { response: responseHtml }, status: 'ok' });
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(payload);
    });
    return;
  }
  res.writeHead(404);
  res.end('not found');
});

server.listen(port, '127.0.0.1', () => {
  console.log(`mock solver listening on http://127.0.0.1:${port}`);
});



