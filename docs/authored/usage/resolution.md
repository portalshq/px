# Resolution
### 3.1 Subtree Resolution

`resolve` extracts exactly the data you need from deep manifest trees — no full-file parsing required:

```bash
# Get the first scene a character appears in
px resolve px://toystory/character/woody references.appears_in.0

# Get just image hashes across all characters (for caching)
px resolve px://toystory/character/woody representations.reference_image.hash

# List available keys for tab completion / introspection
px resolve px://toystory/character/woody#representations

# Different output formats
px resolve px://toystory/character/woody properties -f json
px resolve px://toystory/character/woody properties -f yaml
```

**Use case — AI story generator:** A GPT agent builds a scene by querying the setting, participants, and mood, then generates appropriate dialog — all from fragment queries:

```bash
# Agent gathers context into variables
SETTING=$(px resolve px://toystory/scene/pizza-planet properties -f json)
MOOD=$(px resolve px://toystory/scene/pizza-planet properties.mood -f json)
PARTICIPANTS=$(px resolve px://toystory/scene/pizza-planet properties.participants -f json)

# Agent generates scene using only the relevant data
echo "Setting: $SETTING"
echo "Participants: $PARTICIPANTS"
```

**Use case — API response size optimization:** A mobile client fetching character info only needs the `properties` subtree, not the full manifest (which may include provenance data, representations metadata, references arrays, etc.):

```bash
px resolve px://toystory/character/woody properties -f json
# Returns ~200 bytes instead of ~2000
```

### 3.3 Commit History as Audit Trail

Every change is content-addressed and versioned. Trace exactly how a character evolved and who made each change:

```bash
# View commit history
px history px://toystory/character/woody -n 20
# a72c9f3 2026-06-09T20:15:00Z — set toy_type to human — alice
# b83d1a2 2026-06-09T20:10:00Z — set homeworld — alice
# c94e2b1 2026-06-09T20:05:00Z — added reference_image — bob
# d05f3c0 2026-06-09T20:00:00Z — Create character 'Woody' — alice

# Resolve what the manifest looked like at a specific commit
px resolve px://toystory/character/woody --commit b83d1a2

# View history via API
curl http://localhost:3100/history/toystory/character/woody
```

**Use case — canon dispute resolution:** When two writers disagree on whether Woody's hair color changed between drafts, the commit log shows exactly when and by whom it was modified:

```bash
px history px://toystory/character/woody | grep "hair"
# f7a2b1c 2026-06-08T14:30:00Z — set hair_color to brown — bob
# e8d3c2b 2026-06-07T09:15:00Z — set hair_color to blond — alice
```

**Use case — rollback:** Revert a character to a known good state:

```bash
px revert --commit b83d1a2 toystory
```

### 3.4 Cross-Repository Discovery

Discover what repositorys and entities are available:

```bash
# List all repositorys in the base directory
px list
# px://toystory/
# px://toystory/
# px://middleearth/

# List all entities in a repository
px list toystory
# character:
#   px://toystory/character/woody
#   px://toystory/character/slinky
# location:
#   px://toystory/location/andys-room
#   px://toystory/location/deathstar
# scene:
#   px://toystory/scene/pizza-planet

# Filter by type
px list toystory -t character
# character:
#   px://toystory/character/woody
#   px://toystory/character/slinky
```

---
