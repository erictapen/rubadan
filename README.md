<!--
SPDX-FileCopyrightText: 2026 Kerstin Humm <kerstin@erictapen.name>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# Rubadan

A GUI import tool for [hledger](https://hledger.org/), a plain text accounting tool, usable on the web and on desktop.

Rubadan (short for Rule based data annotation) allows you to visually create rules that assign hledger accounts to your bookkeeping transactions from a bank statement.

The rule language is very close to spoken english, e.g. valid rules are

- When Name contains Patreon then mark as expenses:patreon


## Usage

The easiest way to use Rubadan [is in the browser](https://erictapen.name/rubadan/). There is no server-side data storage, all entered information is either ephemeral or stored in the browser session.

For running it locally you will need to compile it yourself currently:

### Using Nix with Flakes enabled

```
nix run github:erictapen/rubadan
```

### Using cargo

You will need some non-crate dependencies installed like `wayland`/`libx11`.

```
cargo install rubadan
```


## Future for the tool

I wrote Rubadan for my Bachelor's thesis in UI Design at University of Applied Sciences Potsdam and found it to have reached a state that was useful enough for publishing.

Wether I continue development will largely depend on wether I'll start using it for my own finances, so don't expect much to change here until further notice.
