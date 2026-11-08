# Plugin commerce design

This document describes a future commerce service. Payment processing, token issuance, account restoration, revocation distribution, and usage billing are not implemented by this design. The optimizer and training plugins are free.

## Offline entitlement

A plugin manifest will declare a free, one-time, or usage-based commerce model. Free plugins need no token. For a paid plugin, PX will verify an Ed25519 signature against a pinned public key and check the plugin ID, tier, major version, and any expiry in the signed claims. Tokens will be stored under `~/.px/licenses/`; the offline license commands will manage and inspect them. A token is an entitlement, not a secret to put in a repository or plugin manifest.

A one-time purchase grants access to one major version. A future issuer may provide an upgrade grace window by issuing claims that cover the eligible version and time period; clients must enforce the signed claims rather than infer eligibility from a local clock or purchase history. Key rotation and the exact token encoding require a versioned specification before paid plugins ship.

## Future service

A service could restore purchases by email after verifying control of the address, issue signed entitlements, and provide a signed revocation list. Clients should retain the last verified list and fail open if the service or a newer list is unavailable. The service could collect consented activation telemetry to understand usage, without hardware fingerprinting.

Usage-based plans require a server-side ledger: local usage events are only inputs to reconciliation, not an authoritative billing record. Events need stable identifiers, meter units, a timestamp, and plugin identity so retries can be deduplicated. The planned local no-op meter will not bill or persist usage.

A future Stripe integration would validate webhook signatures, deduplicate events, reconcile payment and refund state, and issue or update entitlements after successful payment. It must not trust browser redirects as proof of payment. No Stripe integration or commerce server is part of the current scaffold.

## Signed plugin index

A distribution index can associate plugin IDs and versions with artifact URLs and BLAKE3 hashes. A versioned signature over the exact index bytes, verified using a pinned Ed25519 public key, authenticates the index; the artifact hash verifies the downloaded bytes. Verification must happen before installation. Index hosting, key rotation, rollback protection, and operational procedures remain future work.

## Offline token commands

`px license status` reports token validity without exposing signatures. `px license add <token-file>` verifies a JSON token before saving it; `px license remove <plugin-id>` removes the stored token. Commands run offline and never activate a billing flow.

Issuer keys must first be pinned administratively at `~/.px/licenses/trusted-keys/<plugin-id>.pub` (32 Ed25519 public-key bytes encoded as hexadecimal). Adding a token cannot add or replace the trusted key. Tokens are stored in `~/.px/licenses/<plugin-id>.json`. One-time manifests check the declared commerce major; usage manifests check the plugin version's major. Valid claims are passed to the child only after host verification. Missing, expired, mismatched, or invalid tokens exclude paid plugins; free plugins always remain available.
