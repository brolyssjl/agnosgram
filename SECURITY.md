# Security policy

## Supported versions

Only the latest `1.x` release is supported. Security fixes land on `main`
and ship in the next tagged release; there are no backported patch releases
for older tags.

## Reporting a vulnerability

Report privately through GitHub's private vulnerability reporting on this
repository's **Security** tab (Security -> Report a vulnerability). Do not
open a public issue for a suspected vulnerability.

You should get an acknowledgement within 7 days. From there we'll work with
you on a fix and a disclosure timeline before anything is made public.

## Scope

Agnosgram's [trust posture](docs/trust-posture.md) spells this out in full;
the short version:

- Agnosgram never trusts its own store beyond the structured frontmatter it
  validates. A store file that changes the CLI's behavior through anything
  other than validated frontmatter fields (ids, types, dates, budgets) is in
  scope, as is a prompt-injection payload that `doctor` or a prompt emitter
  fails to warn-and-mark.
- No telemetry, ever - the CLI never phones home and never calls an LLM. Any
  code path that would change that is in scope.
- The install and release pipeline (`install.sh`, `.github/workflows/*`) is
  in scope: checksum or provenance bypass, unpinned actions, or shell
  injection via untrusted input.
- Repository settings (branch/tag rulesets, secret scanning) are owner
  configuration, not code, but a gap there that undermines the above is
  still worth reporting.

## Verifying a release

`install.sh` verifies every downloaded binary against the release's
`SHA256SUMS`, then (when `gh` is installed) against its build-provenance
attestation via `gh attestation verify` - see
[docs/install.md](docs/install.md#provenance-verification) for how to run
either check by hand.
