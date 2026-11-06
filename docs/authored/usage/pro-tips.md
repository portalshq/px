# Pro Tips
### Composition

```bash
# Initialize a repository (defaults to ~/.px/)
px init myrepository

# Override with any directory
px init myrepository -d /path/to/shared/repositorys
px init myrepository -d ~/Dropbox/TeamWorldbuilding

# Point the resolver at a specific path
px -d /mnt/nas/repositorys list
px -d /mnt/nas/repositorys resolve px://toystory/character/woody
```

### Output Formats

```bash
# JSON for programmatic consumption
px resolve px://toystory/character/woody -f json | jq '.properties'

# YAML for human review and editing
px resolve px://toystory/character/woody -f yaml
```

### Debugging

```bash
# Verbose mode shows tracing output
px -v resolve px://toystory/character/woody

# Check server health
curl http://localhost:3100/health
# {"status":"ok","protocol":"PX","version":"0.1.0"}
```

### Quick Bootstrap a New Repository

```bash
# One-liner to seed a new repository
px init mynovel \
  && px create character hero -u mynovel -n "The Hero" \
  && px create location village -u mynovel -n "Home Village" \
  && px set px://mynovel/character/hero homeworld "px://mynovel/location/village" \
  && px set px://mynovel/character/hero archetype "reluctant hero"
```

### Working with Portals

[Portals](https://portals.works) is a team-supported solution powered by PX for creative production.

- Teams can collaborate on shared assets and organizational context.
- Managed version control and creative-production history help teams follow changes across a project.
- Managed production workflows and infrastructure support teams as projects grow.

---
