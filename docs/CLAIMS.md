# Claims — Single Source of Truth

This file is the **canonical wording** for every externally visible claim about
what LocProof does. When a claim changes, change it here first, then propagate
verbatim to the surfaces listed below. Audits should diff against this file.

## Canonical tagline / description

> Tamper-resistant digital witness. Cryptographic evidence that two parties were
> physically present at the same location. Strongly resistant to spoofing through
> multi-signal correlation and cryptographic attestation. Not immune to
> sophisticated collusion or advanced relay attacks without additional external
> anchors.

**Short form** (GitHub repo description, social cards — first three sentences,
the qualifier may be dropped where length is constrained):

> Tamper-resistant digital witness. Cryptographic evidence that two parties were
> physically present at the same location. Strongly resistant to spoofing through
> multi-signal correlation and cryptographic attestation.

### Where the tagline appears
| Surface | Form |
| --- | --- |
| `README.md` (header) | Full |
| GitHub repo description | Short |
| `dashboard/app/page.tsx` (landing hero) | Short |
| `dashboard/app/layout.tsx` (`metadata.description`) | Short |

## Canonical device attestation requirements

Mirrored in `docs/PROTOCOL.md` (§ Device Attestation Requirements):

- iOS: App Attest attestation (Hardware-backed where available) **MUST** be included and validated.
- Android: Play Integrity API verdict (MEETS_DEVICE_INTEGRITY + MEETS_STRONG_INTEGRITY) **MUST** be included.
- Proofs from unattested or failed-integrity devices **MUST be rejected** (fail closed), not merely flagged.
- Attestation verification is step 2 in the proof verification algorithm, before signal scoring.

## Forbidden phrases

These overclaim and must never appear on any surface (`.md`, `.rs`, `.ts`,
`.tsx`):

- "impossible to fake"
- "tamper-proof" (use **tamper-resistant**)
- "court-admissible" / "Court-admissible" (standalone, without a qualifying
  disclaimer)
- "blockchain-anchored" (as an unqualified capability claim)

### Audit command

This file is excluded because it lists the forbidden phrases by definition.

```bash
grep -rn -E "impossible to fake|court-admissible|Court-admissible|tamper-proof|Tamper-proof|blockchain-anchored" \
  --include="*.md" --include="*.ts" --include="*.tsx" --include="*.rs" \
  --exclude="CLAIMS.md" . | grep -v node_modules
```

Expected result: **zero matches**.

## Honest limitations (must stay discoverable)

LocProof reduces but does not eliminate forgery risk. The two unsolved-at-the-
protocol-level threats are documented in `docs/THREAT_MODEL.md`:

- **Collusion** between two cooperating devices — not fully solvable without a
  trusted external anchor.
- **Relay attacks on BLE** — RSSI correlation is only a partial mitigation;
  UWB time-of-flight and mutual RSSI consistency are planned.
