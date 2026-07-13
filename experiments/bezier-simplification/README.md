# Bézier simplification experiment

This prototype explores reducing adjacent cubic Bézier segments while
preserving the visible shape of SVG outlines. Candidate replacements are
scored using sampled deviation, tangent continuity, curvature direction,
enclosed area, and approximate stroke width.

It is intentionally an experiment rather than a production Ferrovia plugin.
The next step would be to define fixtures and error bounds before porting the
algorithm to Rust or exposing it through an optimizer configuration.

## Run

Install the isolated dependencies with `npm ci`, then run `simplify.mjs` with
the input and output arguments described by its command-line usage.
