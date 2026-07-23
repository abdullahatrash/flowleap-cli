#!/usr/bin/env node
/*=============================================================================
 * Dashboard template: CITATION IMPACT (forward-citation distribution)
 *
 * QUESTION  How influential is a patent? The distribution of the patents that
 *           cite it (forward citations) — by examiner category and over time.
 * DATA      `flowleap citation forward "<cited document>" --json`
 *           Response: { success, total, data:[{ applicationNumber, category(X/Y/A),
 *           rejectedClaims, officeActionDate }], citedDocument }.
 *           Source: USPTO Enriched Citations (Office Actions Oct 2017–present).
 *
 * HOW TO ADAPT
 *   1. Pin CITED_DOCUMENT to the exact publication/patent number.
 *   2. Run `node generate.mjs`.
 *
 * NOTE  The citation envelope carries no edition field, so the dataset-identity
 *       row renders the explicit "dataset identity unavailable" (rule 5). This
 *       template does NOT need PATSTAT.
 *===========================================================================*/

// ---- PINNED CONSTANTS (edit these) -----------------------------------------
const CITED_DOCUMENT = 'US 20150284881 A1';
const SUBJECT_SLUG = 'citation-impact'; // subject-based, no timestamps
// ----------------------------------------------------------------------------

import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { mkdirSync, writeFileSync } from 'node:fs';

/*======================= COMMON BLOCK (copy-in) ==============================
 * Identical across the four dashboard templates: CLI subprocess seam, inline
 * SVG chart helpers, HTML shell, and the provenance/reproduce footer.
 *===========================================================================*/

const OUT_DIR = dirname(fileURLToPath(import.meta.url));
const DATA_DIR = join(OUT_DIR, 'data');
const FLOWLEAP_BIN = process.env.FLOWLEAP_CLI_BIN || 'flowleap';

/** Error carrying the parsed FlowLeap envelope so callers can branch on code. */
class CliError extends Error {
	constructor(message, { code = null, status = null, args = [], candidates = null } = {}) {
		super(message);
		this.code = code;
		this.status = status;
		this.args = args;
		this.candidates = candidates;
	}
}

/** Read the machine `error.code` out of a failed-run envelope (client wrap or bare body). */
function envelopeError(parsed) {
	if (!parsed || typeof parsed !== 'object') return null;
	const body = parsed.body && typeof parsed.body === 'object' ? parsed.body : parsed;
	const err = body.error && typeof body.error === 'object' ? body.error : null;
	if (!err) return null;
	return { code: err.code ?? null, message: err.message ?? '', candidates: err.candidates ?? null };
}

/** Run `flowleap <args> --json` as a subprocess and return the parsed body. Never holds a token. */
function runCli(args) {
	try {
		const out = execFileSync(FLOWLEAP_BIN, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
		return JSON.parse(out);
	} catch (e) {
		const stdout = typeof e.stdout === 'string' ? e.stdout : '';
		let parsed = null;
		try { parsed = JSON.parse(stdout); } catch { /* not JSON */ }
		const info = envelopeError(parsed);
		if (info) {
			throw new CliError(info.message || `flowleap ${args.join(' ')} failed`, {
				code: info.code, status: e.status ?? null, args, candidates: info.candidates,
			});
		}
		if (e.code === 'ENOENT') {
			throw new CliError(`Could not spawn "${FLOWLEAP_BIN}". Install the FlowLeap CLI or set FLOWLEAP_CLI_BIN.`, { args });
		}
		throw new CliError((e.stderr || e.message || 'unknown error').toString().trim(), { status: e.status ?? null, args });
	}
}

/** Run a subprocess expecting plain text (e.g. `flowleap --version`). */
function runCliText(args) {
	return execFileSync(FLOWLEAP_BIN, args, { encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 }).trim();
}

/** Persist one raw-JSON sidecar per backend call under ./data/. */
function saveSidecar(name, value) {
	mkdirSync(DATA_DIR, { recursive: true });
	writeFileSync(join(DATA_DIR, name), JSON.stringify(value, null, 2));
	return `data/${name}`;
}

const fmtInt = (n) => Number(n ?? 0).toLocaleString('en-US');
const svgEsc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const htmlEsc = svgEsc;

// Okabe-Ito colorblind-safe categorical palette; single-hue ramp for heatmaps.
const OKABE_ITO = ['#0072B2', '#E69F00', '#009E73', '#D55E00', '#CC79A7', '#56B4E9', '#F0E442', '#000000'];
const BLUES = ['#deebf7', '#9ecae1', '#6baed6', '#3182bd', '#08519c'];
const AXIS = '#333', GRID = '#e2e2e2', INK = '#1a1a1a', MUTE = '#5a5a5a';

/** Round an axis maximum up to a readable value. */
function niceMax(v) {
	if (v <= 0) return 1;
	const p = Math.pow(10, Math.floor(Math.log10(v)));
	const f = v / p;
	const n = f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10;
	return n * p;
}

function chartFrame(title, source, w, h, inner) {
	return `<svg viewBox="0 0 ${w} ${h}" width="100%" role="img" aria-label="${svgEsc(title)}" style="max-width:${w}px;background:#fff;font-family:system-ui,-apple-system,Segoe UI,Roboto,sans-serif">
<title>${svgEsc(title)}</title>
<text x="16" y="24" fill="${INK}" font-size="16" font-weight="600">${svgEsc(title)}</text>
<text x="16" y="42" fill="${MUTE}" font-size="11">Source: ${svgEsc(source)}</text>
${inner}
</svg>`;
}

function legend(x, y, series) {
	return series.map((s, i) => {
		const cy = y + i * 18;
		return `<rect x="${x}" y="${cy - 9}" width="11" height="11" fill="${OKABE_ITO[i % OKABE_ITO.length]}"/>` +
			`<text x="${x + 16}" y="${cy}" fill="${INK}" font-size="11">${svgEsc(s.name)}</text>`;
	}).join('');
}

/** Grouped or stacked bar chart. series = [{name, values:[]}] aligned to categories. */
function barChart({ title, source, categories, series, yLabel = 'Count', stacked = false }) {
	const W = 760, H = 420, m = { t: 64, r: 150, b: 70, l: 64 };
	const pw = W - m.l - m.r, ph = H - m.t - m.b;
	const max = niceMax(stacked
		? Math.max(1, ...categories.map((_, ci) => series.reduce((a, s) => a + (s.values[ci] || 0), 0)))
		: Math.max(1, ...series.flatMap((s) => s.values)));
	const y = (v) => m.t + ph - (v / max) * ph;
	const bandW = pw / Math.max(categories.length, 1);
	const ticks = 4;
	let g = '';
	for (let i = 0; i <= ticks; i++) {
		const v = (max / ticks) * i, yy = y(v);
		g += `<line x1="${m.l}" y1="${yy}" x2="${m.l + pw}" y2="${yy}" stroke="${GRID}"/>` +
			`<text x="${m.l - 8}" y="${yy + 4}" fill="${MUTE}" font-size="10" text-anchor="end">${fmtInt(v)}</text>`;
	}
	let bars = '';
	categories.forEach((cat, ci) => {
		const x0 = m.l + ci * bandW;
		if (stacked) {
			let acc = 0;
			series.forEach((s, si) => {
				const v = s.values[ci] || 0; if (!v) return;
				const yTop = y(acc + v), hgt = y(acc) - y(acc + v);
				bars += `<rect x="${x0 + bandW * 0.18}" y="${yTop}" width="${bandW * 0.64}" height="${Math.max(hgt, 0)}" fill="${OKABE_ITO[si % OKABE_ITO.length]}"><title>${svgEsc(cat)} - ${svgEsc(s.name)}: ${fmtInt(v)}</title></rect>`;
				acc += v;
			});
		} else {
			const gw = (bandW * 0.72) / Math.max(series.length, 1);
			series.forEach((s, si) => {
				const v = s.values[ci] || 0;
				const bx = x0 + bandW * 0.14 + si * gw;
				bars += `<rect x="${bx}" y="${y(v)}" width="${Math.max(gw - 2, 1)}" height="${m.t + ph - y(v)}" fill="${OKABE_ITO[si % OKABE_ITO.length]}"><title>${svgEsc(cat)} - ${svgEsc(s.name)}: ${fmtInt(v)}</title></rect>`;
			});
		}
		bars += `<text x="${x0 + bandW / 2}" y="${m.t + ph + 16}" fill="${MUTE}" font-size="10" text-anchor="middle">${svgEsc(cat)}</text>`;
	});
	const axes = `<line x1="${m.l}" y1="${m.t}" x2="${m.l}" y2="${m.t + ph}" stroke="${AXIS}"/>` +
		`<line x1="${m.l}" y1="${m.t + ph}" x2="${m.l + pw}" y2="${m.t + ph}" stroke="${AXIS}"/>` +
		`<text transform="translate(16,${m.t + ph / 2}) rotate(-90)" fill="${MUTE}" font-size="11" text-anchor="middle">${svgEsc(yLabel)}</text>`;
	return chartFrame(title, source, W, H, g + axes + bars + legend(m.l + pw + 20, m.t + 6, series));
}

/** Multi-series line chart over an ordered category axis (e.g. years). */
function lineChart({ title, source, categories, series, yLabel = 'Count' }) {
	const W = 760, H = 420, m = { t: 64, r: 150, b: 70, l: 64 };
	const pw = W - m.l - m.r, ph = H - m.t - m.b;
	const max = niceMax(Math.max(1, ...series.flatMap((s) => s.values)));
	const y = (v) => m.t + ph - (v / max) * ph;
	const x = (i) => m.l + (categories.length <= 1 ? pw / 2 : (i / (categories.length - 1)) * pw);
	const ticks = 4;
	let g = '';
	for (let i = 0; i <= ticks; i++) {
		const v = (max / ticks) * i, yy = y(v);
		g += `<line x1="${m.l}" y1="${yy}" x2="${m.l + pw}" y2="${yy}" stroke="${GRID}"/>` +
			`<text x="${m.l - 8}" y="${yy + 4}" fill="${MUTE}" font-size="10" text-anchor="end">${fmtInt(v)}</text>`;
	}
	categories.forEach((cat, i) => {
		g += `<text x="${x(i)}" y="${m.t + ph + 16}" fill="${MUTE}" font-size="10" text-anchor="middle">${svgEsc(cat)}</text>`;
	});
	let lines = '';
	series.forEach((s, si) => {
		const col = OKABE_ITO[si % OKABE_ITO.length];
		const pts = s.values.map((v, i) => `${x(i)},${y(v)}`).join(' ');
		lines += `<polyline points="${pts}" fill="none" stroke="${col}" stroke-width="2.5"/>`;
		lines += s.values.map((v, i) => `<circle cx="${x(i)}" cy="${y(v)}" r="3.2" fill="${col}"><title>${svgEsc(s.name)} ${svgEsc(categories[i])}: ${fmtInt(v)}</title></circle>`).join('');
	});
	const axes = `<line x1="${m.l}" y1="${m.t}" x2="${m.l}" y2="${m.t + ph}" stroke="${AXIS}"/>` +
		`<line x1="${m.l}" y1="${m.t + ph}" x2="${m.l + pw}" y2="${m.t + ph}" stroke="${AXIS}"/>` +
		`<text transform="translate(16,${m.t + ph / 2}) rotate(-90)" fill="${MUTE}" font-size="11" text-anchor="middle">${svgEsc(yLabel)}</text>`;
	return chartFrame(title, source, W, H, g + axes + lines + legend(m.l + pw + 20, m.t + 6, series));
}

/** Heatmap. rows/cols are labels; matrix[r][c] is the value. Empty cells (0) flag white space. */
function heatmap({ title, source, rows, cols, matrix, valueLabel = 'Filings' }) {
	const cell = 46, labelW = 96, top = 64, left = labelW + 8;
	const W = Math.max(760, left + cols.length * cell + 24);
	const H = top + rows.length * cell + 96;
	const max = Math.max(1, ...matrix.flat());
	const colorFor = (v) => (v <= 0 ? '#f5f5f5' : BLUES[Math.min(BLUES.length - 1, Math.floor((v / max) * BLUES.length - 1e-9))]);
	let cells = '';
	cols.forEach((c, ci) => {
		cells += `<text x="${left + ci * cell + cell / 2}" y="${top - 8}" fill="${MUTE}" font-size="10" text-anchor="middle">${svgEsc(c)}</text>`;
	});
	rows.forEach((r, ri) => {
		cells += `<text x="${labelW}" y="${top + ri * cell + cell / 2 + 4}" fill="${INK}" font-size="11" text-anchor="end">${svgEsc(r)}</text>`;
		cols.forEach((c, ci) => {
			const v = matrix[ri][ci] || 0;
			const x = left + ci * cell, yv = top + ri * cell;
			cells += `<rect x="${x}" y="${yv}" width="${cell - 2}" height="${cell - 2}" fill="${colorFor(v)}" stroke="#fff"><title>${svgEsc(r)} / ${svgEsc(c)}: ${fmtInt(v)} ${svgEsc(valueLabel)}</title></rect>`;
			cells += `<text x="${x + cell / 2 - 1}" y="${yv + cell / 2 + 3}" fill="${v / max > 0.6 ? '#fff' : INK}" font-size="10" text-anchor="middle">${v === 0 ? '·' : fmtInt(v)}</text>`;
		});
	});
	// Discrete swatch legend (no gradient — chartjunk-free).
	const ly = top + rows.length * cell + 28;
	let leg = `<text x="${left}" y="${ly}" fill="${MUTE}" font-size="10">Low</text>`;
	['#f5f5f5', ...BLUES].forEach((col, i) => {
		leg += `<rect x="${left + 30 + i * 22}" y="${ly - 10}" width="20" height="12" fill="${col}" stroke="#ccc"/>`;
	});
	leg += `<text x="${left + 30 + 6 * 22 + 4}" y="${ly}" fill="${MUTE}" font-size="10">High &nbsp; (leftmost = white space)</text>`;
	return chartFrame(title, source, W, H, cells + leg);
}

/**
 * Provenance + reproduce footer. `sources` is one descriptor per backend call:
 * { verb, params, datasetIdentity, retrievedAt, sidecar }. A null datasetIdentity
 * renders the explicit "dataset identity unavailable" row (never omitted).
 */
function renderFooter({ generatedAt, cliVersion, backend, sources, reproduce }) {
	const rows = sources.map((s) => `<li>
		<code>${htmlEsc(s.verb)}</code> <span class="params">${htmlEsc(s.params)}</span><br>
		<span class="lbl">Dataset identity:</span> ${s.datasetIdentity ? htmlEsc(s.datasetIdentity) : '<em>dataset identity unavailable</em>'}<br>
		<span class="lbl">Retrieved:</span> ${htmlEsc(s.retrievedAt)}
		${s.sidecar ? ` &middot; <a href="${htmlEsc(s.sidecar)}">${htmlEsc(s.sidecar)}</a>` : ''}
	</li>`).join('\n');
	return `<footer id="provenance" data-provenance="footer">
	<h2>Provenance &amp; Reproduce</h2>
	<p class="meta"><span class="lbl">Generated:</span> ${htmlEsc(generatedAt)}
	 &middot; <span class="lbl">CLI / backend:</span> ${htmlEsc(cliVersion)} / ${htmlEsc(backend)}</p>
	<h3>Data sources</h3>
	<ul class="sources">${rows}</ul>
	<h3>Reproduce</h3>
	<pre class="reproduce">${htmlEsc(reproduce)}</pre>
	<p class="note">Every figure above is computed in <code>generate.mjs</code> from the linked
	<code>data/</code> sidecars — none is typed by hand. Regenerate by re-running the script.</p>
</footer>`;
}

/** Assemble the self-contained HTML document (inline CSS, no external assets). */
function htmlDoc({ title, subtitle, summary, bodyHtml, footerHtml }) {
	return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${htmlEsc(title)}</title>
<style>
:root { color-scheme: light; }
* { box-sizing: border-box; }
body { margin: 0; background: #f4f5f7; color: ${INK};
	font-family: system-ui,-apple-system,Segoe UI,Roboto,sans-serif; line-height: 1.5; }
main { max-width: 900px; margin: 0 auto; padding: 32px 20px 64px; }
header.page h1 { margin: 0 0 4px; font-size: 24px; }
header.page .sub { color: ${MUTE}; margin: 0 0 8px; }
.summary { background: #fff; border-left: 4px solid ${OKABE_ITO[0]}; padding: 12px 16px;
	border-radius: 6px; margin: 16px 0 24px; }
section.card { background: #fff; border: 1px solid #e6e6e6; border-radius: 10px;
	padding: 18px 20px; margin: 0 0 20px; overflow-x: auto; }
section.card h2 { margin: 0 0 4px; font-size: 17px; }
section.card p.lead { color: ${MUTE}; margin: 0 0 12px; font-size: 14px; }
table { border-collapse: collapse; width: 100%; font-size: 14px; }
th, td { text-align: right; padding: 6px 10px; border-bottom: 1px solid #eee; }
th:first-child, td:first-child { text-align: left; }
thead th { color: ${MUTE}; font-weight: 600; border-bottom: 2px solid #ddd; }
footer#provenance { background: #fff; border: 1px solid #e6e6e6; border-radius: 10px;
	padding: 18px 20px; font-size: 13px; color: #333; }
footer#provenance h2 { font-size: 16px; margin: 0 0 8px; }
footer#provenance h3 { font-size: 13px; text-transform: uppercase; letter-spacing: .04em;
	color: ${MUTE}; margin: 16px 0 6px; }
footer .lbl { color: ${MUTE}; font-weight: 600; }
footer ul.sources { list-style: none; padding: 0; margin: 0; }
footer ul.sources li { padding: 8px 0; border-bottom: 1px dashed #eee; }
footer code { background: #f0f2f4; padding: 1px 5px; border-radius: 4px; }
footer .params { color: #444; }
pre.reproduce { background: #1a1a1a; color: #eee; padding: 12px 14px; border-radius: 8px;
	overflow-x: auto; font-size: 12.5px; }
.caveat { color: #8a5a00; font-size: 13px; margin: 8px 0 0; }
</style>
</head>
<body>
<main>
<header class="page">
	<h1>${htmlEsc(title)}</h1>
	<p class="sub">${htmlEsc(subtitle)}</p>
</header>
${summary ? `<div class="summary">${htmlEsc(summary)}</div>` : ''}
${bodyHtml}
${footerHtml}
</main>
</body>
</html>`;
}

/** Print the chat summary block: computed numbers + file paths, for the agent to paste. */
function printSummaryBlock(lines, htmlPath) {
	const out = ['```', ...lines, `Dashboard: ${htmlPath}`, `(open locally: open "${htmlPath}")`, '```'];
	console.log(out.join('\n'));
}

/*======================= END COMMON BLOCK ===================================*/

/** Impact narrative — every number interpolated from computed `c` (rule 1). */
function renderNarrative(c) {
	const domCat = Object.entries(c.byCategory).sort((a, b) => b[1] - a[1])[0];
	const CAT_MEANING = { X: 'novelty-destroying (35 USC 102)', Y: 'obviousness (35 USC 103)', A: 'background' };
	const peak = c.byYearArr.reduce((a, b) => (b.count > a.count ? b : a), c.byYearArr[0] || { year: '—', count: 0 });
	const parts = [
		`${htmlEsc(c.citedDocument)} has ${fmtInt(c.total)} forward citations in USPTO office actions.`,
		domCat ? `Most are ${domCat[0]}-category (${CAT_MEANING[domCat[0]] || domCat[0]}): ${fmtInt(domCat[1])} citations.` : '',
		peak && peak.count ? `Citing activity peaked in ${peak.year} (${fmtInt(peak.count)} citations).` : '',
		c.byCategory.X ? `${fmtInt(c.byCategory.X)} are X-category — the strongest signal this patent is potent prior art.` : '',
	];
	return parts.filter(Boolean).join(' ');
}

function main() {
	const generatedAt = new Date().toISOString();
	let cliVersion = 'unknown', backend = 'unknown';
	try { cliVersion = runCliText(['--version']); } catch { /* best effort */ }
	try {
		const d = runCli(['--json', 'doctor']);
		backend = d.baseUrl ? `${d.baseUrl} (health: ${d?.backend?.healthStatus ?? 'unknown'})` : 'unknown';
	} catch { /* best effort */ }

	const args = ['--json', 'citation', 'forward', CITED_DOCUMENT];
	const res = runCli(args);
	const sidecar = saveSidecar('citation-forward.json', res);
	const rows = res.data ?? [];

	// Compute distributions from the raw citations (rule 1).
	const byCategory = { X: 0, Y: 0, A: 0 };
	const yearMap = new Map();
	for (const r of rows) {
		const cat = (r.category || '').toUpperCase();
		if (cat in byCategory) byCategory[cat] += 1;
		const yr = String(r.officeActionDate || '').slice(0, 4);
		if (/^\d{4}$/.test(yr)) yearMap.set(yr, (yearMap.get(yr) || 0) + 1);
	}
	const years = [...yearMap.keys()].sort();
	const byYearArr = years.map((y) => ({ year: y, count: yearMap.get(y) }));
	const c = { citedDocument: res.citedDocument ?? CITED_DOCUMENT, total: res.total ?? rows.length, byCategory, byYearArr };

	const catChart = barChart({
		title: 'Forward Citations by Examiner Category',
		source: 'flowleap citation forward (USPTO Enriched Citations)',
		categories: ['X', 'Y', 'A'],
		series: [{ name: 'Citations', values: ['X', 'Y', 'A'].map((k) => byCategory[k]) }], yLabel: 'Citations',
	});
	const yearChart = lineChart({
		title: 'Forward Citations Over Time',
		source: 'flowleap citation forward (USPTO Enriched Citations; office-action year)',
		categories: years, series: [{ name: 'Citations', values: byYearArr.map((r) => r.count) }], yLabel: 'Citations',
	});
	const CAT_MEANING = { X: 'Novelty-destroying (102)', Y: 'Obviousness (103)', A: 'Background' };
	const catRows = ['X', 'Y', 'A'].map((k) => `<tr><td>${k} — ${htmlEsc(CAT_MEANING[k])}</td><td>${fmtInt(byCategory[k])}</td></tr>`).join('');

	const bodyHtml = `
<section class="card"><h2>Impact Summary</h2><p class="lead">${renderNarrative(c)}</p></section>
<section class="card"><h2>By Examiner Category</h2>${catChart}
<table><thead><tr><th>Category</th><th>Citations</th></tr></thead><tbody>${catRows}</tbody></table></section>
<section class="card"><h2>Over Time</h2>${yearChart}</section>`;

	const reproduce = [
		`# Cited document pinned once with the user:`,
		`#   CITED_DOCUMENT = "${CITED_DOCUMENT}"`,
		`flowleap --json citation forward "${CITED_DOCUMENT}"`,
		'node generate.mjs',
	].join('\n');

	const footerHtml = renderFooter({
		generatedAt, cliVersion, backend,
		sources: [{
			verb: 'citation forward', params: `"${CITED_DOCUMENT}"`,
			datasetIdentity: null, // USPTO Enriched Citations carries no edition field in the envelope
			retrievedAt: generatedAt, sidecar,
		}],
		reproduce,
	});
	const html = htmlDoc({
		title: `Citation Impact — ${c.citedDocument}`,
		subtitle: `Forward citations from USPTO office actions · ${fmtInt(c.total)} citing references`,
		summary: '', bodyHtml, footerHtml,
	});
	const htmlPath = join(OUT_DIR, 'dashboard.html');
	writeFileSync(htmlPath, html);
	printSummaryBlock([
		`Citation impact — ${c.citedDocument}`,
		`Forward citations: ${fmtInt(c.total)}  (X: ${byCategory.X}, Y: ${byCategory.Y}, A: ${byCategory.A})`,
	], htmlPath);
}
main();
