add custom draw escape hatch for low-level rendering

Surfaces can now execute batched DrawOp operations (rects, text, lines,
ellipses, images, rounded rects) on top of the retained-mode scene graph.
One-shot by design; the next scene graph change overwrites custom draw
content. Includes a Rust bar chart example.
