# ghg — git history as a waveform

`ghg` turns a repository's commit history into a full-screen waveform. Time runs
left to right, every commit is a bar, bar height is how much that commit changed,
and colour is a heat ramp over the same value. Quiet weeks become valleys, a day
of refactoring becomes a ridge, and one giant commit glows near the top of the
ramp.

```
ghg · wokamark.nvim
main · 21 commits · 2 authors · HEAD · blend axis · log heat
heat ████████████████████████  low → 712 lines → high

█                   █
█   ▁               █            ▄                                             █
█   █         █     █            █                                             █
█   █         █     █            █                                             █
█   █       ▂ █     █            █                                             █
█   █ ▃     █ █ ▅ ▁ █      █     █   ▅ ▄                                       █
█ ▄ █ █     █ █ █ █ █  █   █ █▄  █ ▅ █ █                                       █
█ █ █ █   ▄ █ █ █ █ █  █ █ █ ██  █ █ █ █                                       █
█ █ █ █ █ █ █ █ █ █ █  █ █ █ ██  █ █ █ █                                       █
█ █ █ █ █ █ █ █ █ █ █  █ █ █ ██  █ █ █ █                                       █
┴────────────────────────┴───────────┴─────────────────────────────────────────┴
08-07                    08-07       08-08

█ █████████  065a45a docs: split README int…  LingLing …  3mo        +258  -73
  ┄┄ 1 week ┄┄
▒ ████       3e31c91 docs: trim design-note…  LingLing …  3mo          +3   -9
▒ ████       eeca868 fix: load_entry idempo…  LingLing …  3mo         +11   -2
▒ ███        ed2ad59 fix: call M.open_tree_…  LingLing …  3mo          +5   -1
█ ████████   ad3db3b Add GPLv3 license       LingLing …  3mo        +232   -0
```

*(That is the `--plain` rendering; the interactive TUI adds an info card,
a scrollable list, a detail pane and mouse support.)*

## Install

```sh
cargo install --path .        # or: cargo build --release && ./target/release/ghg
```

Requires `git` on `PATH`. There is no libgit2 dependency: history is read with a
single `git log --numstat` call.

## Usage

```sh
ghg                     # current repository
ghg ~/src/thing         # any path inside a repository
ghg --all               # every branch, not just HEAD
ghg -n 500              # only the newest 500 commits
ghg --since 2024-01-01  # git --since syntax
ghg --author ada        # filter by author
ghg --theme viridis --scale linear
ghg --plain             # static output, safe for pipes/pagers
ghg --plain --no-color | less
ghg --dump-config       # print the effective config
ghg --list-themes
```

When stdout is not a terminal, `ghg` automatically uses plain mode. Colours
degrade from truecolor to 256 to 16 colours depending on `COLORTERM` / `TERM`,
or use `--no-color` / `--color`.

## Keys

| key | action |
| --- | --- |
| `↑` `↓` / `k` `j` | select previous / next commit |
| `PgUp` `PgDn` | page through the list |
| `g` `G` | oldest / newest |
| `←` `→` / `h` `l` | pan the window |
| `+` `-` | zoom in / out around the selection |
| `0` / `Esc` | reset zoom |
| `↵` | toggle the detail pane (side-by-side when wide) |
| `v` | open the full commit record: message, stat and patch (scroll, `q` closes) |
| `/` | search; non-matching commits are dimmed, not hidden |
| `n` `N` | next / previous match |
| `t` | sort: newest → largest → smallest |
| `s` | toggle log / linear heat |
| `a` | all branches / current branch |
| `T` | cycle themes |
| `L` | hide / show the commit list |
| `R` | reload config and repository |
| `m` | toggle mouse |
| `?` | help overlay |
| `q` / `Ctrl-C` | quit |

Esc also clears an applied search. Clicking a bar selects its commit; the wheel
scrolls the list.

### Seeing one commit in full

Select a commit (arrow keys, click, or `/` search) and press `v`. A full-screen
viewer opens with the complete record: hash, author and committer dates, the
entire commit message, the `--stat` summary and the patch, coloured green/red
per hunk. Scroll with `j`/`k` or `PgUp`/`PgDn`, jump with `g`/`G`, and close
with `v`, `q` or `Esc` — quitting the viewer leaves `ghg` running.

Large patches are capped at 8000 lines and end with a truncation marker. The
same data is available from git directly:

```sh
git show <hash>          # header + message + patch
git show --stat <hash>   # summary only
```

## How the heat is computed

* **Value** — insertions + deletions from `git log --numstat`. Binary files
  count as zero.
* **Reference** — the 95th percentile of those values (`heat_percentile`), not
  the maximum, so one imported vendor dump cannot flatten everything else.
* **Mapping** — `log(1 + value) / log(1 + reference)`, then an optional gamma.
  `heat_scale = linear` switches to plain proportion.
* **Height** — the *same* normalised value used for the colour, drawn with
  eighth blocks (`▁▂▃▄▅▆▇█`) for sub-cell precision. Wave bars, the compact
  bar in the commit list, and every per-file bar in the detail pane all share
  this one value and reference: a 10-line file in a quiet commit cannot light
  up like a giant one. Colour is a five-stop ramp interpolated over it.
* **Merges** — git does not report a diffstat for them, so they are drawn as a
  one-eighth mark in the dim colour and labelled `merge`.

The **x axis** has two modes:

* `blend` (default) — half wall-clock, half commit order. A burst of 30 commits
  in one evening stays readable while quiet stretches still take room.
* `time` — strictly proportional to wall-clock time.

Axis labels follow the mode: calendar boundaries in `time`, evenly spaced
commits in `blend`.

## Configuration

`ghg` reads `~/.config/ghg/ghg.conf` (XDG aware), creates it with annotated
defaults on first run, and reloads it with `R`. Override the location with
`--config FILE` or `GHG_CONFIG`; skip it with `--no-config`.

The format is INI-ish and btop-flavoured:

```ini
[general]
theme = inferno            # built-in name, custom theme name, or a path
limit = 5000               # 0 = all commits
all_branches = false
relative_time = true       # false -> use date_format
date_format = %Y-%m-%d
mouse = true
wave_height = 0.55
wave_mode = volume         # volume | density | both
wave_axis = blend          # blend | time
density_strip = true
heat_scale = log           # log | linear
heat_percentile = 95
heat_gamma = 1.0
glyphs = eighth            # eighth | block | ascii
bar_width = 20
row_fields = heat,bar,hash,subject,author,time,diff,refs
detail_layout = auto       # auto | side | bottom
gap_hours = 12             # insert a gap separator when idle this long

[colors]
heat_low = "#1e1b4b"
heat_1 = "#6d28d9"
heat_2 = "#db2777"
heat_3 = "#f97316"
heat_high = "#fde047"
# fg, dim, accent, ok, bad, warn, axis, border, selected_bg, wave_peak

[keys]
zoom_in = +
zoom_out = -
help = ?
# any action name from `--dump-config` neighbour list
```

Unknown keys produce a warning shown in the header (`⚠ n config warnings`)
instead of aborting.

### Themes

Built in: `inferno`, `magma`, `viridis`, `ice`, `mono`, `solarized`.

Custom themes live in `~/.config/ghg/themes/<name>.theme` and can use either
btop syntax or a `[colors]` section:

```
theme[heat_low] = "#101322"
theme[heat_high] = "#ffd166"
```

or

```ini
[colors]
heat_low = #101322
heat_high = #ffd166
accent = #ffd166
```

`ghg --list-themes` prints the built-in ramps and the theme directory.

## Plain mode

`--plain` (or piping) prints a header, a 10-row waveform, the axis, and the
commit list with ANSI colours. It is deterministic: no alternate screen, no
mouse, no timers. Useful for `less -R`, screenshots, CI logs, or embedding in
other tools.

## Development

```sh
cargo test          # unit tests + end-to-end CLI tests in a temp repo
cargo build --release
```

Source layout:

```
src/git.rs      git log invocation + numstat parser
src/model.rs    Commit / Repo / refs
src/stats.rs    percentile, normalisation, projection, buckets, ticks
src/rows.rs     shared commit-row renderer (TUI + plain)
src/theme.rs    ramps, role colours, colour-depth degradation
src/config.rs   config parsing, defaults, key maps
src/ui/         waveform, list, detail, chrome, help, event loop
src/plain.rs    non-interactive rendering
```
