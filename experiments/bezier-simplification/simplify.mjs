import { readFileSync, writeFileSync } from "fs";
import { JSDOM } from "jsdom";
import SVGPath from "svgpath";

// --- Bezier math ---

function bezierPoint(p0, cp1, cp2, p3, t) {
  const mt = 1 - t;
  return [
    mt ** 3 * p0[0] + 3 * mt ** 2 * t * cp1[0] + 3 * mt * t ** 2 * cp2[0] + t ** 3 * p3[0],
    mt ** 3 * p0[1] + 3 * mt ** 2 * t * cp1[1] + 3 * mt * t ** 2 * cp2[1] + t ** 3 * p3[1],
  ];
}

function bezierCurvature(p0, cp1, cp2, p3, t) {
  const mt = 1 - t;
  const dx = 3 * mt ** 2 * (cp1[0] - p0[0]) + 6 * mt * t * (cp2[0] - cp1[0]) + 3 * t ** 2 * (p3[0] - cp2[0]);
  const dy = 3 * mt ** 2 * (cp1[1] - p0[1]) + 6 * mt * t * (cp2[1] - cp1[1]) + 3 * t ** 2 * (p3[1] - cp2[1]);
  const ddx = 6 * mt * (cp2[0] - 2 * cp1[0] + p0[0]) + 6 * t * (p3[0] - 2 * cp2[0] + cp1[0]);
  const ddy = 6 * mt * (cp2[1] - 2 * cp1[1] + p0[1]) + 6 * t * (p3[1] - 2 * cp2[1] + cp1[1]);
  const denom = (dx * dx + dy * dy) ** 1.5;
  if (denom < 1e-10) return 0;
  return (dx * ddy - dy * ddx) / denom;
}

function dist(a, b) {
  return Math.sqrt((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2);
}

function normalize(v) {
  const len = Math.sqrt(v[0] ** 2 + v[1] ** 2);
  if (len < 1e-10) return [1, 0];
  return [v[0] / len, v[1] / len];
}

// --- Sample and measure ---

function sampleSegment(p0, cp1, cp2, p3, n = 20) {
  const pts = [];
  for (let i = 0; i <= n; i++) pts.push(bezierPoint(p0, cp1, cp2, p3, i / n));
  return pts;
}

function maxDeviation(candidateP0, candidateCp1, candidateCp2, candidateP3, refPoints) {
  let maxDev = 0;
  for (const rp of refPoints) {
    let minD = Infinity;
    for (let i = 0; i <= 50; i++) {
      const bp = bezierPoint(candidateP0, candidateCp1, candidateCp2, candidateP3, i / 50);
      const d = dist(rp, bp);
      if (d < minD) minD = d;
    }
    if (minD > maxDev) maxDev = minD;
  }
  return maxDev;
}

// --- Fit replacement bezier preserving tangent directions ---

function fitReplacementBezier(p0, p3, tanOutDir, tanInDir, refPoints) {
  const chord = dist(p0, p3);
  if (chord < 1e-6) return { cp1: p0, cp2: p3, deviation: 0 };

  let bestDev = Infinity;
  let bestCp1, bestCp2;

  const coarseSteps = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0];
  let bestA1 = 0.33, bestA2 = 0.33;

  for (const a1 of coarseSteps) {
    for (const a2 of coarseSteps) {
      const cp1 = [p0[0] + a1 * chord * tanOutDir[0], p0[1] + a1 * chord * tanOutDir[1]];
      const cp2 = [p3[0] - a2 * chord * tanInDir[0], p3[1] - a2 * chord * tanInDir[1]];
      const dev = maxDeviation(p0, cp1, cp2, p3, refPoints);
      if (dev < bestDev) {
        bestDev = dev;
        bestA1 = a1;
        bestA2 = a2;
        bestCp1 = cp1;
        bestCp2 = cp2;
      }
    }
  }

  for (let a1 = bestA1 - 0.1; a1 <= bestA1 + 0.1; a1 += 0.02) {
    for (let a2 = bestA2 - 0.1; a2 <= bestA2 + 0.1; a2 += 0.02) {
      if (a1 <= 0 || a2 <= 0) continue;
      const cp1 = [p0[0] + a1 * chord * tanOutDir[0], p0[1] + a1 * chord * tanOutDir[1]];
      const cp2 = [p3[0] - a2 * chord * tanInDir[0], p3[1] - a2 * chord * tanInDir[1]];
      const dev = maxDeviation(p0, cp1, cp2, p3, refPoints);
      if (dev < bestDev) {
        bestDev = dev;
        bestCp1 = cp1;
        bestCp2 = cp2;
      }
    }
  }

  return { cp1: bestCp1, cp2: bestCp2, deviation: bestDev };
}

// --- Width measurement ---
// For a closed path (brushstroke outline), measure the local "stroke width"
// by finding the minimum distance from a point to the "opposite side" of the shape.
// The opposite side = segments that are far away in path-parameter-space.

function measureMinWidthAtPoint(point, segs, segIndex, totalSegs) {
  let minD = Infinity;
  const halfSegs = Math.floor(totalSegs / 2);

  for (let i = 0; i < totalSegs; i++) {
    // Skip segments near the current one (within 2 segments in either direction)
    const fwdDist = (i - segIndex + totalSegs) % totalSegs;
    const bwdDist = (segIndex - i + totalSegs) % totalSegs;
    const pathDist = Math.min(fwdDist, bwdDist);
    if (pathDist < 3) continue;

    const s = segs[i];
    for (let t = 0; t <= 1; t += 0.1) {
      const p = bezierPoint(s.start, s.cp1, s.cp2, s.end, t);
      const d = dist(point, p);
      if (d < minD) minD = d;
    }
  }
  return minD;
}

// Measure width profile along two segments (before removal)
function measureWidthProfile(segs, segIdx1, segIdx2, totalSegs) {
  const widths = [];
  for (const idx of [segIdx1, segIdx2]) {
    const s = segs[idx];
    for (const t of [0.25, 0.5, 0.75]) {
      const p = bezierPoint(s.start, s.cp1, s.cp2, s.end, t);
      const w = measureMinWidthAtPoint(p, segs, idx, totalSegs);
      if (w < Infinity) widths.push(w);
    }
  }
  return widths;
}

// Measure width profile along replacement segment
function measureReplacementWidthProfile(p0, cp1, cp2, p3, segs, replacedIdx, totalSegs) {
  const widths = [];
  for (const t of [0.2, 0.35, 0.5, 0.65, 0.8]) {
    const p = bezierPoint(p0, cp1, cp2, p3, t);
    const w = measureMinWidthAtPoint(p, segs, replacedIdx, totalSegs);
    if (w < Infinity) widths.push(w);
  }
  return widths;
}

// --- Area computation ---

function computePathArea(startPos, segs) {
  const pts = [];
  let pos = startPos;
  for (const s of segs) {
    const sampled = sampleSegment(s.start || pos, s.cp1, s.cp2, s.end, 10);
    const startIdx = pts.length === 0 ? 0 : 1;
    for (let i = startIdx; i < sampled.length; i++) pts.push(sampled[i]);
    pos = s.end;
  }
  let area = 0;
  for (let i = 0; i < pts.length; i++) {
    const j = (i + 1) % pts.length;
    area += pts[i][0] * pts[j][1];
    area -= pts[j][0] * pts[i][1];
  }
  return Math.abs(area / 2);
}

// --- Core: iterative single-node removal ---

function simplifyCubicRun(startPos, cubics, tolerance, isClosed) {
  if (cubics.length <= 1) return cubics;
  const minSegments = isClosed ? Math.max(3, Math.ceil(cubics.length * 0.4)) : 2;

  // Build mutable node list
  let nodes = [];

  nodes.push({
    pos: startPos,
    outDir: normalize([cubics[0].cp1[0] - startPos[0], cubics[0].cp1[1] - startPos[1]]),
    outDist: dist(startPos, cubics[0].cp1),
    inDir: null,
    inDist: 0,
  });

  for (let i = 0; i < cubics.length; i++) {
    const c = cubics[i];
    nodes.push({
      pos: c.end,
      outDir: i < cubics.length - 1
        ? normalize([cubics[i + 1].cp1[0] - c.end[0], cubics[i + 1].cp1[1] - c.end[1]])
        : null,
      outDist: i < cubics.length - 1 ? dist(c.end, cubics[i + 1].cp1) : 0,
      inDir: normalize([c.end[0] - c.cp2[0], c.end[1] - c.cp2[1]]),
      inDist: dist(c.cp2, c.end),
    });
  }

  let segs = cubics.map((c, i) => ({
    cp1: c.cp1, cp2: c.cp2, end: c.end,
    start: i === 0 ? startPos : cubics[i - 1].end,
  }));

  const originalArea = computePathArea(startPos, segs);
  // Only do width checks if path has enough segments to have meaningful "opposite" sides
  const doWidthCheck = isClosed && segs.length >= 8;

  let changed = true;
  while (changed && segs.length > minSegments) {
    changed = false;
    let bestCost = Infinity;
    let bestIdx = -1;
    let bestFit = null;

    for (let k = 1; k < nodes.length - 1; k++) {
      const prev = nodes[k - 1];
      const curr = nodes[k];
      const next = nodes[k + 1];

      if (curr.protected) continue;

      // Check tangent continuity
      if (curr.inDir && curr.outDir) {
        const dot = curr.inDir[0] * curr.outDir[0] + curr.inDir[1] * curr.outDir[1];
        if (dot < 0.7) continue;
      }

      const seg1 = segs[k - 1];
      const seg2 = segs[k];
      const refPoints = [
        ...sampleSegment(seg1.start, seg1.cp1, seg1.cp2, seg1.end, 15),
        ...sampleSegment(seg2.start, seg2.cp1, seg2.cp2, seg2.end, 15).slice(1),
      ];

      const tanOut = prev.outDir;
      const tanIn = next.inDir;
      if (!tanOut || !tanIn) continue;

      const fit = fitReplacementBezier(prev.pos, next.pos, tanOut, tanIn, refPoints);

      // Curvature consistency check
      const origCurvEnd = bezierCurvature(seg1.start, seg1.cp1, seg1.cp2, seg1.end, 1.0);
      const origCurvStart = bezierCurvature(seg2.start, seg2.cp1, seg2.cp2, seg2.end, 0.0);
      const newCurvMid = bezierCurvature(prev.pos, fit.cp1, fit.cp2, next.pos, 0.5);
      const origSign = Math.sign(origCurvEnd) === Math.sign(origCurvStart);
      const curvaturePenalty = origSign && Math.sign(newCurvMid) !== Math.sign(origCurvEnd) ? 2.0 : 0;

      // Width preservation check
      let widthPenalty = 0;
      if (doWidthCheck) {
        const widthsBefore = measureWidthProfile(segs, k - 1, k, segs.length);
        const minWidthBefore = Math.min(...widthsBefore);

        if (minWidthBefore > 3 && minWidthBefore < Infinity) {
          const widthsAfter = measureReplacementWidthProfile(
            prev.pos, fit.cp1, fit.cp2, next.pos, segs, k - 1, segs.length
          );
          const minWidthAfter = Math.min(...widthsAfter);

          if (minWidthAfter < Infinity) {
            const widthLoss = (minWidthBefore - minWidthAfter) / minWidthBefore;
            if (widthLoss > 0.20) {
              // More than 20% width loss — heavy penalty
              widthPenalty = 10.0;
            } else if (widthLoss > 0.10) {
              // 10-20% width loss — moderate penalty
              widthPenalty = widthLoss * 15;
            }
          }
        }
      }

      const cost = fit.deviation + curvaturePenalty + widthPenalty;

      if (cost < bestCost && fit.deviation <= tolerance && widthPenalty < 5.0) {
        bestCost = cost;
        bestIdx = k;
        bestFit = fit;
      }
    }

    if (bestIdx >= 0) {
      const prev = nodes[bestIdx - 1];
      const next = nodes[bestIdx + 1];
      const oldSegs = segs.slice();

      const newSeg = {
        start: prev.pos,
        cp1: bestFit.cp1,
        cp2: bestFit.cp2,
        end: next.pos,
      };

      segs.splice(bestIdx - 1, 2, newSeg);

      // Area preservation check
      if (isClosed && originalArea > 10) {
        const newArea = computePathArea(startPos, segs);
        const areaChange = Math.abs(newArea - originalArea) / originalArea;
        if (areaChange > 0.15) {
          segs.length = 0;
          segs.push(...oldSegs);
          nodes[bestIdx].protected = true;
          changed = true;
          continue;
        }
      }

      nodes.splice(bestIdx, 1);

      prev.outDir = normalize([bestFit.cp1[0] - prev.pos[0], bestFit.cp1[1] - prev.pos[1]]);
      prev.outDist = dist(prev.pos, bestFit.cp1);
      next.inDir = normalize([next.pos[0] - bestFit.cp2[0], next.pos[1] - bestFit.cp2[1]]);
      next.inDist = dist(bestFit.cp2, next.pos);

      changed = true;
    }
  }

  return segs.map((s) => ({ cp1: s.cp1, cp2: s.cp2, end: s.end }));
}

// --- Path processing ---

function parseSegments(d) {
  const parsed = SVGPath(d).abs().unshort();
  const segments = [];
  parsed.iterate((seg) => segments.push(seg));
  return segments;
}

function simplifyPath(d, tolerance) {
  const segments = parseSegments(d);
  const result = [];
  let i = 0;
  let currentPos = [0, 0];
  let totalBefore = 0;
  let totalAfter = 0;

  while (i < segments.length) {
    const seg = segments[i];
    const cmd = seg[0];

    if (cmd === "M") {
      currentPos = [seg[1], seg[2]];
      result.push(seg);
      i++;
      continue;
    }

    if (cmd === "C") {
      const cubics = [];
      while (i < segments.length && segments[i][0] === "C") {
        const s = segments[i];
        cubics.push({ cp1: [s[1], s[2]], cp2: [s[3], s[4]], end: [s[5], s[6]] });
        i++;
      }

      const isClosed = i < segments.length && (segments[i][0] === "Z" || segments[i][0] === "z");

      totalBefore += cubics.length;
      const simplified = simplifyCubicRun(currentPos, cubics, tolerance, isClosed);
      totalAfter += simplified.length;

      for (const c of simplified) {
        const r = (v) => Math.round(v * 10) / 10;
        result.push(["C", r(c.cp1[0]), r(c.cp1[1]), r(c.cp2[0]), r(c.cp2[1]), r(c.end[0]), r(c.end[1])]);
      }

      currentPos = cubics[cubics.length - 1].end;
      continue;
    }

    if (cmd === "L") currentPos = [seg[1], seg[2]];
    else if (cmd === "Q") currentPos = [seg[3], seg[4]];
    else if (cmd === "A") currentPos = [seg[6], seg[7]];
    result.push(seg);
    i++;
  }

  const pathStr = result.map((seg) => seg[0] + seg.slice(1).join(",")).join("");
  return { path: pathStr, before: totalBefore, after: totalAfter };
}

// --- Main ---

const [,, inputFile, outputFile, toleranceStr] = process.argv;

if (!inputFile || !outputFile) {
  console.log("Usage: node simplify.mjs <input.svg> <output.svg> [tolerance]");
  console.log("  tolerance: max deviation in SVG units (default: 2.0)");
  process.exit(1);
}

const tolerance = parseFloat(toleranceStr || "2.0");
const svgContent = readFileSync(inputFile, "utf-8");
const dom = new JSDOM(svgContent, { contentType: "image/svg+xml" });
const doc = dom.window.document;

const paths = doc.querySelectorAll("path");
let totalBefore = 0;
let totalAfter = 0;

for (const pathEl of paths) {
  const d = pathEl.getAttribute("d");
  if (!d) continue;
  const style = pathEl.getAttribute("style") || "";
  if (style.includes("fill:none")) continue;

  const { path: simplified, before, after } = simplifyPath(d, tolerance);
  pathEl.setAttribute("d", simplified);
  totalBefore += before;
  totalAfter += after;
}

const serializer = new dom.window.XMLSerializer();
let output = serializer.serializeToString(doc.documentElement);
output = '<?xml version="1.0" encoding="UTF-8" standalone="no"?>\n' + output;
writeFileSync(outputFile, output, "utf-8");

const reduction = totalBefore > 0 ? ((1 - totalAfter / totalBefore) * 100).toFixed(1) : 0;
console.log(`Done!`);
console.log(`  Cubic segments: ${totalBefore} → ${totalAfter} (${reduction}% reduction)`);
console.log(`  Tolerance: ${tolerance} SVG units`);
console.log(`  Output: ${outputFile}`);
