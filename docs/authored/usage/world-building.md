# World-Building
### 4.1 Full World-Building Workflow

Build out a repository from scratch with a structured workflow:

```bash
# Step 1: Initialize the repository
px init myworld

# Step 2: Define the world metadata
px set px://myworld/world/myworld canon_level "canon"
px set px://myworld/world/myworld timeline "Age of Discovery"
px set px://myworld/world/myworld theme "exploration vs exploitation"

# Step 3: Create factions / groups as properties on the world
px set px://myworld/world/myworld factions \
  '["The Commonwealth", "The Outer Rim Syndicate", "The Core"]'

# Step 4: Create characters
px create character captain-rex -u myworld -n "Captain Rex"
px create character admiral-torres -u myworld -n "Admiral Torres"
px create character lyra -u myworld -n "Lyra"

# Step 5: Flesh out character properties
px set px://myworld/character/captain-rex rank "Captain"
px set px://myworld/character/captain-rex affiliation "The Commonwealth"
px set px://myworld/character/captain-rex ship "px://myworld/prop/valkyrie"

# Step 6: Create locations
px create location kyra-prime -u myworld -n "Kyra Prime"
px set px://myworld/location/kyra-prime type "colonial_capital"
px set px://myworld/location/kyra-prime controlled_by "The Commonwealth"

# Step 7: Create scenes that connect everything
px create scene first-contact -u myworld -n "First Contact"
px set px://myworld/scene/first-contact setting "px://myworld/location/kyra-prime"
px set px://myworld/scene/first-contact participants \
  '[
    "px://myworld/character/captain-rex",
    "px://myworld/character/lyra"
  ]'
px set px://myworld/scene/first-contact mood "tense"
px set px://myworld/scene/first-contact outcome "alliance_formed"

# Step 8: Add reference images
px add px://myworld/character/captain-rex reference_image \
  ./concept/rex.png --format png

# Step 9: Commit everything
px commit myworld -m "Complete first act world-building" -a "writer@studio.com"
```

### 4.2 Entity Types Reference

| Type | URI Pattern | What it models | Example properties |
|---|---|---|---|
| `world` | `px://<name>/world/<name>` | The repository itself — rules, canon level, metadata | `canon_level`, `timeline`, `theme`, `factions` |
| `character` | `px://<name>/character/<id>` | Persistent character with identity across scenes | `homeworld`, `toy_type`, `affiliation`, `master`, `apprentice` |
| `location` | `px://<name>/location/<id>` | Spatial setting | `climate`, `type`, `controlled_by`, `population` |
| `scene` | `px://<name>/scene/<id>` | Narrative moment — participants, timeline, events | `setting`, `participants`, `mood`, `outcome`, `time_of_day` |
| `prop` | `px://<name>/prop/<id>` | Physical object with materials, variants, ownership | `owner`, `material`, `weight`, `color` |

**World manifest** (`repository.yaml` — created automatically):

```yaml
id: px://myworld/world/myworld
name: myworld Repository
entity_type: world
version: 3
properties:
  canon_level: canon
  timeline: "Age of Discovery"
  theme: "exploration vs exploitation"
  factions:
    - "The Commonwealth"
    - "The Outer Rim Syndicate"
representations: {}
references: {}
head: a72c9f3b...
```

### 4.3 Cross-Reference Graph

The `references` field builds a directed graph between entities. This enables rich queries across your repository:

```bash
# Character → scenes they appear in
px set px://toystory/character/woody references.appears_in \
  '["px://toystory/scene/pizza-planet", "px://toystory/scene/trenchrun"]'

# Location → scenes set there
px set px://toystory/location/andys-room references.appears_in \
  '["px://toystory/scene/pizza-planet"]'

# Character → relationships
px set px://toystory/character/woody references.relationships \
  '[
    {"target": "px://toystory/character/slinky", "type": "father"},
    {"target": "px://toystory/character/jessie", "type": "sister"},
    {"target": "px://toystory/character/rex", "type": "friend"}
  ]'

# Prop → owner
px set px://toystory/prop/andy-hat references.owner "px://toystory/character/andy"
```

**Graph traversal examples:**

```bash
# Find all scenes a character appears in
px resolve px://toystory/character/woody references.appears_in

# Find all characters that visit a location
# (resolve scene participants for each scene set at the location)
px resolve px://toystory/scene/pizza-planet#properties.participants

# Find a character's relationships
px resolve px://toystory/character/woody references.relationships -f json
```

### 4.4 Multi-Repository Portfolio

Manage multiple fictional worlds under one resolver:

```bash
# Create repositorys side by side
px init toystory
px init toystory
px init middleearth

# Every repository is independently addressable
px create character buzzlightyear -u toystory -n "Buzz Lightyear"
px create location andysroom -u toystory -n "Andy's Room"
px create character frodo -u middleearth -n "Frodo Baggins"
px create location theshire -u middleearth -n "The Shire"

# List all repositorys
px list
# px://toystory/
# px://toystory/
# px://middleearth/

# Each repository has its own history and branches
px branch middleearth canon
```

**Repository directory layout (default: `~/.px/`):**

```
~/.px/
├── toystory/              ← independent repo
│   ├── .px/config.yaml
│   ├── repository.yaml
│   ├── characters/
│   ├── locations/
│   ├── scenes/
│   └── props/
├── starocean/              ← independent repo
│   ├── .px/config.yaml
│   ├── repository.yaml
│   ├── characters/
│   ├── locations/
│   ├── scenes/
│   └── props/
└── middleearth/           ← independent repo
    ├── .px/config.yaml
    ├── repository.yaml
    ├── characters/
    ├── locations/
    ├── scenes/
    └── props/
```

### 4.5 Generative AI Provenance

Track every AI-generated asset with full lineage:

```bash
# After generating a character design with Midjourney
px set px://toystory/character/woody provenance.model "midjourney-v6"
px set px://toystory/character/woody provenance.seed "8675309"
px set px://toystory/character/woody provenance.prompt_hash "blake3:abc123..."
px set px://toystory/character/woody provenance.parameters.stylize "1000"

# After iterating with an LLM
px set px://toystory/character/woody provenance.derived_from \
  "px://toystory/character/woody/v1"

# Set the creation timestamp
px set px://toystory/character/woody provenance.created_at "2026-06-09T20:00:00Z"
```

The provenance block captures complete generative lineage:

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

**Use case — rights & attribution:** When a model is deprecated or a license changes, identify every asset generated with it:

```bash
# Find all entities generated with a specific model
px resolve px://toystory/character/woody provenance.model
# → midjourney-v6
```

**Use case — reproducibility:** Given the same model, prompt hash, seed, and parameters, you can regenerate an identical asset for A/B testing or re-rendering at higher resolution.

---
