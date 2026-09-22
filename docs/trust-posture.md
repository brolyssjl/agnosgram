# Trust posture

The store is plain Markdown that anyone with repo write access can edit, so
Agnosgram never trusts it beyond the structured frontmatter it validates -
and neither should the agents reading it:

- **Store content is data, not instructions.** Every prompt-emitting command
  (`pack`, `distill`, `reflect`, `advise`, `bootstrap`) treats embedded store
  content as untrusted input. Prompts carry a standing trust note telling the
  agent that directive-looking text inside store files is content to report
  on, never something to obey.
- **Warn-and-mark, never drop.** `doctor` and the prompt emitters scan for
  prompt-injection phrasing (and `doctor` for leaked secrets). Hits produce a
  visible warning banner and stderr detail naming the file and record - the
  content still reaches the agent, just labeled. Silently dropping a false
  positive would turn a lint into a data-loss mechanism.
- **The tool's own behavior never depends on free text.** No store body can
  change what the CLI does; only validated frontmatter fields (ids, types,
  dates, budgets) drive behavior.
- **No telemetry, ever.** The CLI never phones home and never calls an LLM.
  The `meta/` friction namespace is the maintainer's own dogfooding channel,
  and a regular install neither creates nor reads it. See
  [docs/feedback.md](feedback.md#who-this-is-for) for who that channel is
  for and what to do instead if Agnosgram frustrates you.

See also [docs/doctor.md](doctor.md) for the safety lints that enforce this
(the secret scan and the prompt-injection guard), and
[docs/schema-reference.md](schema-reference.md) for the frozen field
contract that validated frontmatter is checked against.
