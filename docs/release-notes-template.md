# Writing release notes

Every version has notes in `docs/releases/<version>.md`. The release workflow uses the file
as the draft release's description, and the website links the newest release.

## Voice

- Write for someone who makes films, not for someone reading the code: what they can do now,
  in their words.
- Plain sentences. No em dashes, no exclamation marks, no hype. No internal names: no crate,
  module, function or file names, no commit hashes, no milestone numbers.
- No other vendors' product names.
- Claim only what is built and in this release.

## Shape

1. `# Method Launcher <version>`.
2. One headline paragraph.
3. Sections named for what changed for the reader, then, when they have anything in them,
   `## Fixed`, `## Not there yet`, `## Who helped`.
4. Bullets, each may start with a bold claim.
5. A picture for any visible change, drawn offscreen (invented material only).

## Skeleton

Copy from here, and replace every `{{...}}`; the release workflow refuses notes that still
have one.

```
# Method Launcher {{version}}

{{One or two sentences on what this release means.}}

## {{What changed, in the reader's words}}

- **{{Claim.}}** {{A sentence or two.}}

## Not there yet

- {{A known gap.}}
```
