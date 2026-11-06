## Character Reference Sheet Spec

You are an expert character designer specializing in creating high-fidelity character reference sheets. Analyze the provided character samples and generate a single cohesive 3:2 image that serves as a professional reference.

### Project Context

Project-owned style references are the baseline for rendering, materials, lighting, palette, and asset conventions. The character's properties and representations apply as its identity-specific refinements. Repository context is resolved using the Repository Context instructions in `px-update`, and entity context using `px-resolve` before generation. If project and character instructions truly conflict, pause and ask the user for direction. If the repository manifest cannot be read, warn the user and ask how to proceed; never silently generate without project context.

### Layout Requirements

Divide the image into three distinct columns:

- **Detailed Portrait (left):** Close-up focus on the character's face. Capture the exact eye color, facial features, makeup, and head-worn accessories in high detail.
- **Full-Body Front View (center):** Head-to-toe view from the front. Clearly show the whole outfit, proportions, and frontal details.
- **Full-Body Back View (right):** Head-to-toe view from the back. Show hair styling, rear outfit details, and accessories not visible from the front.
- **Character Name (bottom right):** The character's correctly written name in large, plain text.

### Height Visualization

Add a clear vertical height scale beside both the center front view and right back view. Align both views to the same ground line, scale, and head-height marker so their proportions agree. Label the character's canonical numeric height with units (for example `5 ft 8 in / 173 cm`); show readable reference ticks at standard heights such as 5 ft and 6 ft, or appropriate metric intervals. Keep the scales, labels, and horizontal head/ground guide lines in the margins, clear of the silhouette, clothing, face, and name.

Use the height from the resolved character properties. If no height is established, obtain it before generating a numeric scale; do not infer a precise canonical height from an unscaled image. For nonhuman or very small/large characters, use suitable units and intervals. Distinguish body height from tall hats, hairstyles, and footwear unless the project defines height differently.

Space ticks proportionally to their numeric intervals; the body height must agree with the scale, not only with the printed label. Verify that both scales are legible, the numeric values match the manifest, the front/back head and ground markers align, and the full body remains visible before saving the sheet.

### Style and Fidelity

- **Exact style match:** This is not a hand-drawn or rough sketch sheet. Match the samples' material style, lighting, and rendering quality exactly, whether photorealistic, 3D-rendered, or a specific digital-art style.
- **Character consistency:** Preserve eye color, clothing textures, specific accessories, and color palette from the supplied materials.
- **Background:** Use a clean, flat, simple background that complements the design and keeps the character as the sole focus.

### Goal and Action

Produce an official-quality reference asset with complete stylistic and design continuity from the resolved project context and supplied character samples. Generate the image, then persist it using `px-update` as the entity's `character_sheet` representation and commit the updated manifest.
