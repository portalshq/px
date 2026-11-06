# Reference
```bash
# Initialize a repository in ~/.px/
px init toystory

# Create entities
px create character woody -u toystory -n "Woody"
px create location andys-room -u toystory -n "Andy's Room"
px create scene pizza-planet -u toystory -n "Pizza Planet Scene"

# Set properties with cross-references
px set px://toystory/character/woody toy_type human
px set px://toystory/character/woody homeworld "px://toystory/location/andys-room"

# Resolve manifests
px resolve px://toystory/character/woody

# Fragment queries
px resolve px://toystory/character/woody#properties.homeworld

# Subtree resolution
px resolve px://toystory/character/woody properties

# Version control
px history px://toystory/character/woody
px branch toystory canon
px head toystory

```

---
