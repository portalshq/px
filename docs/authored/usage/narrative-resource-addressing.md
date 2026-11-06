# Narrative Resource Addressing
### 1.1 URI Anatomy

Every narrative resource gets a stable, canonical `px://` URI:

```
px://toystory/character/woody#properties.homeworld
────┬── ───┬──── ────┬──── ──────┬────── ─────────────┬───────────
 scheme repository  entity_type entity_id          fragment (query)
```

**Key rules:**
- Branch and commit selectors are **never** in the URI path — pass them alongside the URI instead.
- Fragment (`#`) carries the query path for subtree extraction.
- Entity type is singular in the URI (`character`, not `characters`).

### 1.2 Cross-Reference Grid

Build a web of references between entities using `px://` URIs as values:

```bash
# Create locations
px create location andys-room -u toystory -n "Andy's Room"
px create location alderaan -u toystory -n "Alderaan"
px create location deathstar -u toystory -n "Death Star"

# Create a scene
px create scene pizza-planet -u toystory -n "Pizza Planet Scene"

# Cross-reference everything with px:// URIs
px set px://toystory/character/woody homeworld "px://toystory/location/andys-room"
px set px://toystory/character/woody affiliation "toybox_alliance"
px set px://toystory/character/woody master "px://toystory/character/mrpotatohead"

px set px://toystory/location/andys-room climate "desert"
px set px://toystory/location/andys-room moons "2"

px set px://toystory/scene/pizza-planet setting "px://toystory/location/andys-room"
px set px://toystory/scene/pizza-planet participants \
  '["px://toystory/character/woody", "px://toystory/character/buzzlightyear"]'
```

The resulting manifest for Woody looks like this:

```yaml
id: px://toystory/character/woody
name: Woody
entity_type: character
version: 5
properties:
  homeworld: "px://toystory/location/andys-room"
  toy_type: human
  affiliation: toybox_alliance
  master: "px://toystory/character/mrpotatohead"
references: {}
head: a72c9f3b...
```

**Use case — story bible automation:** A script can traverse every `px://` URI in a manifest and verify it resolves. If someone deletes `px://toystory/character/mrpotatohead`, the broken reference is caught immediately.

### 1.3 Fragment Queries for Precision Addressing

Address sub-parts of a resource with `#fragment` syntax — ideal for AI agents, integration pipelines, and script generators:

```bash
# Get a single field
px resolve px://toystory/character/woody#properties.homeworld
# → "px://toystory/location/andys-room"

# Get a reference array
px resolve px://toystory/scene/pizza-planet#properties.participants

# Chain into nested objects
px resolve px://toystory/character/woody#representations.reference_image.hash

# Array index access
px resolve px://toystory/character/woody#references.appears_in.0
```

**Use case — AI context window optimization:** Instead of feeding an LLM a 40K-token manifest, pull exactly the 500 tokens it needs:

```bash
# An AI writing a scene only needs participants and setting
px resolve px://toystory/scene/pizza-planet properties.participants -f json
px resolve px://toystory/scene/pizza-planet properties.setting -f json
```

**Use case — CI/CD validation:** Verify every cross-reference resolves:

```bash
px resolve px://toystory/character/woody#references.appears_in \
  | jq -r '.[]' \
  | xargs -I{} px resolve {}
```

### 1.4 Versioned Addressing for Canon Management

Branches and commits are **orthogonal selectors** — never in the URI. Address the same resource at different points in its timeline:

```bash
# Create branches for alternate canon tracks
px branch toystory legends
px branch toystory canon
px branch toystory "what-if"

# Resolve at specific points in time
px resolve px://toystory/character/woody --branch legends
px resolve px://toystory/character/woody --commit a72c9f3b
```

**Use case — divergent timelines:** In a "What If" branch, Woody joins the Toybox Corp. The `canon` branch has `affiliation: toybox_alliance`; the `what-if` branch has `affiliation: toybox_corp`. Both resolve from the same URI — only the selector differs. The manifests diverge silently, and the resolver picks the right one based on context.

```bash
px resolve px://toystory/character/woody#properties.affiliation
# → toybox_alliance

px resolve px://toystory/character/woody#properties.affiliation \
  --branch what-if
# → toybox_corp
```

---
