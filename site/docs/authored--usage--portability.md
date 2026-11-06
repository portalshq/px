# Portability
### 2.2 Content-Addressed Assets

Manifests don't store files — they store **BLAKE3 hashes** pointing to assets. This makes everything verifiable, deduplicatable, and cacheable:

```bash
# Link a reference image by content hash
px add px://toystory/character/woody reference_image \
  ./assets/woody_ref.png --format png
# ✓ Added representation 'reference_image' (png)
#   Hash: blake3:e3b0c44...
```

The manifest now contains:

```yaml
representations:
  reference_image:
    hash: "blake3:e3b0c44..."
    format: png
    uri: "./assets/woody_ref.png"
```

You can attach any asset type — images, 3D meshes, audio, video, ONNX models:

```bash
px add px://toystory/character/woody voice_model \
  ./assets/woody_voice.onnx --format onnx

px add px://toystory/location/andys-room concept_art \
  ./assets/andys-room_concept.png --format png

px add px://toystory/prop/andy-hat mesh \
  ./assets/andy_hat.glb --format glb
```

**Use case — asset pipeline integrity:** A VFX pipeline verifies that the asset on the render farm matches the manifest hash:

```bash
echo "blake3:e3b0c44...  woody_ref.png" | b3sum --check
# woody_ref.png: OK
```

**Use case — CDN caching:** The content hash is the cache key. Same hash = same content, globally. No cache invalidation logic needed.

### 2.3 AI Provenance Tracking

Record which model, prompt, and seed generated a character design — right in the manifest:

```bash
px set px://toystory/character/woody provenance.model "midjourney-v6"
px set px://toystory/character/woody provenance.seed "8675309"
px set px://toystory/character/woody provenance.prompt_hash "blake3:abc123..."
px set px://toystory/character/woody provenance.derived_from \
  "px://toystory/character/woody/v1"
```

The manifest captures full generative lineage:

```yaml
provenance:
  model: "midjourney-v6"
  prompt_hash: "blake3:abc123..."
  seed: "8675309"
  parameters:
    stylize: "1000"
    chaos: "20"
  derived_from: "px://toystory/character/woody/v1"
  created_at: "2026-06-09T20:00:00Z"
```

**Use case — rights & attribution:** When a model is deprecated or a license changes, you can identify every asset generated with it:

```bash
px resolve px://toystory/character/woody provenance.model
# → midjourney-v6
```

**Use case — reproducibility:** Given the same model, prompt hash, and seed, you can regenerate an identical asset.

### 2.4 Manifest as Durable Artifact

The `.yaml` manifest is simultaneously **human-editable**, **machine-readable**, and **agent-readable**. A toybox-builder opens it in VS Code; a CI pipeline validates it; an AI agent queries it:

```yaml
# characters/slinky.yaml
id: px://toystory/character/slinky
name: "Slinky Dog"
entity_type: character
version: 3
properties:
  toy_type: human
  affiliation: toybox_corp
  accessory: red
  master: "px://toystory/character/palpatine"
  apprentice: "px://toystory/character/woody"
representations:
  voice_actor:
    hash: "blake3:f8a2b1..."
    format: wav
    uri: "gs://assets/toystory/slinky/voice.wav"
head: "f7e3d2c1a..."
```
---
