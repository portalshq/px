# Design Principles
- **Manifest is current state. History is external.** Manifests store only `head` — a pointer to the latest commit. Full history lives in the VCS, preventing unbounded manifest growth.
- **Branch and commit are NEVER in the URI.** They are orthogonal selectors passed alongside the URI.
- **Content-address everything.** Every representation is identified by its BLAKE3 hash. Manifests are content-hashable for signing and verification.
- **Subtree queries are first-class.** AI systems, CLI tools, and HTTP clients all use the same query engine. Fragment queries enable efficient data access without fetching entire manifests.

---

*PX is in v0 (prototype). The core data model and resolution engine are functional. Advanced features (signing, verification, distributed resolution) are planned for future iterations.*
