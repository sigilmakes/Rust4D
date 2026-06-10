---
name: headless-visual-verification
description: Run autonomous visual checks for Rust4D rendering changes. Use after touching WGSL shaders, RenderUniforms, camera projection, slice pipeline, primitives, or examples that affect rendering.
---

# Headless Visual Verification

Willow does not do manual rendering checks for this project. If a change affects
visual output, verify it yourself with offscreen GPU captures.

## Slice invariant protocol

Use for camera/movement/physics/slice correctness:

```bash
nix develop --command cargo run --example headless_protocol .scratchpad/captures
```

Read the `[STATE] slice_w(...)` logs. During WASD phases, slice W must remain
constant. Convert frames with ImageMagick if needed.

## Primitive showcase protocol

Use for geometry, shader, material, and lighting work:

```bash
nix develop --command cargo run --example shape_showcase .scratchpad/captures-gallery
```

Expected:
- 81 captures (9 primitives × 3 slice offsets × 3 orientations)
- zero zero-triangle captures
- no cracks/T-junctions/hairline seams
- central slices of every primitive visibly distinct

Make a quick contact sheet:

```bash
mkdir -p .scratchpad/captures-gallery/png
for f in .scratchpad/captures-gallery/*_mid_identity.ppm; do
    magick "$f" ".scratchpad/captures-gallery/png/$(basename "${f%.ppm}").png"
done
magick .scratchpad/captures-gallery/png/tesseract_mid_identity.png \
       .scratchpad/captures-gallery/png/hypersphere_mid_identity.png \
       .scratchpad/captures-gallery/png/pentachoron_mid_identity.png +append row1.png
# Repeat rows, then `magick row1.png row2.png row3.png -append contact-sheet.png`
```

Use `+append`/`-append`; ImageMagick `montage` may need fonts unavailable in
the nix shell.

## GPU warning

The current wgpu/naga stack may emit Vulkan validation warning
`VUID-StandaloneSpirv-MemorySemantics-10871` for `OpAtomicIAdd` relaxed
semantics. This is harmless for now and tracked as a wgpu upgrade backlog item.
Do not confuse it with a rendering failure.
