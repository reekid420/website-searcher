// scripts/csrin_search.js
// Usage: node scripts/csrin_search.js "elden ring"
// Requires: npm i -D playwright

const { chromium } = require('playwright');

(async () => {
	const query = process.argv.slice(2).join(' ').trim();
	if (!query) {
		console.error('Missing query');
		process.exit(2);
	}
	const browser = await chromium.launch({ headless: true });
	const context = await browser.newContext({
		userAgent:
			"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
	});
	const cookie = process.env.PLAYWRIGHT_COOKIE;
	if (cookie) {
		// Parse simple cookie header into individual cookies for cs.rin.ru
		// Example: "phpbb3_x=u=1; sid=abc; ..."
		const cookies = cookie.split(';').map(x => x.trim()).filter(Boolean);
		for (const c of cookies) {
			const [name, ...rest] = c.split('=');
			const value = rest.join('=');
			if (!name || !value) continue;
			await context.addCookies([{ name, value, url: 'https://cs.rin.ru/forum/' }]);
		}
	}
	const page = await context.newPage();
	const pagesToScan = Math.max(1, Math.min(parseInt(process.env.CSRIN_PAGES || '2', 10) || 2, 10));
	let haveResults = false;

	// Primary path: use the search form to establish a valid session and search context
	try {
		await page.goto('https://cs.rin.ru/forum/search.php', { waitUntil: 'domcontentloaded', timeout: 45000 });
		// Close donation overlay if present
		try { await page.click('#overlayconfirmbtn', { timeout: 1500 }); } catch {}
		await page.fill('input[name="keywords"]', query);
		await page.selectOption('select[name="sr"]', { value: 'topics' });
		await page.check('input[name="fid[]"][value="10"]');
		await Promise.all([
			page.waitForLoadState('networkidle'),
			page.click('input[name="submit"]'),
		]);
		await page.waitForLoadState('domcontentloaded');
		// Detect rate limiting or missing results
		const infoText = await page.textContent('table.tablebg td.row1 .gen').catch(() => null);
		haveResults = !!(await page.$('a.topictitle').catch(() => null)) && !(infoText && infoText.includes('cannot use search at this time'));
		if (!haveResults) {
			throw new Error('Search unavailable or empty, falling back to listing pages');
		}
		const html = await page.content();
		console.log(html);
		await browser.close();
		process.exit(0);
	} catch (_) {
		// Fallback: direct URL build first, if still blocked then scan listing pages with pagination
		try {
			const params = new URLSearchParams();
			params.set('keywords', query);
			params.set('sr', 'topics');
			params.append('fid[]', '10');
			const url = `https://cs.rin.ru/forum/search.php?${params.toString()}`;
			await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 45000 });
			try { await page.waitForSelector('a.topictitle', { timeout: 5000 }); } catch {}
			const infoText = await page.textContent('table.tablebg td.row1 .gen').catch(() => null);
			haveResults = !!(await page.$('a.topictitle').catch(() => null)) && !(infoText && infoText.includes('cannot use search at this time'));
			if (haveResults) {
				const html = await page.content();
				console.log(html);
				await browser.close();
				process.exit(0);
			}
		} catch {}

		// Final fallback: scan listing pages (f=10) with pagination
		let combined = '';
		for (let i = 0; i < pagesToScan; i++) {
			const url = `https://cs.rin.ru/forum/viewforum.php?f=10&start=${i * 100}`;
			await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 45000 });
			try { await page.waitForSelector('a.topictitle', { timeout: 5000 }); } catch {}
			combined += await page.content();
		}
		console.log(combined);
		await browser.close();
		process.exit(0);
	}
	await browser.close();
	process.exit(0);
})().catch((e) => {
	console.error(String(e && e.stack || e));
	process.exit(1);
});
