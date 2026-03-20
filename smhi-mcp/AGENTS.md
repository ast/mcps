# AGENTS.md

This project will be a Claude Code/LLM connector to the Swedish
weather forcaster SMHI.

The goal is to provide forcast data so I know how to dress and what to
expect for the day.

It will be written in Rust.

- It will use the API from SMHI as documented here:
  https://www.smhi.se/data/sok-oppna-data-i-utforskaren/

- It will use the offical Rust MCP SDK:
  https://github.com/modelcontextprotocol/rust-sdk


## Crates and SDK

It will use best quality crates as far as possible.

It will use anyhow for Result and Context and thiserror for errors.
