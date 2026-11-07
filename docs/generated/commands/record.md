---
generated: "true"
generator: px-docgen
version: 0.10.0
source: clap
---


# px record
Record generated outputs, their exact prompt, and provenance together


## Synopsis
```bash
px record [OPTIONS] --branch <BRANCH> --output <OUTPUT> --prompt <PROMPT> <URI>
```


## Arguments

| Name | Description | Required |
|---|---|---|
| uri | PX entity URI | Yes |


## Options

| Flag | Description | Default |
|---|---|---|
|     --branch | Branch currently checked out in the repository |  |
|     --input-refs | JSON array of input references with URI, key, hash, and commit | [] |
|     --job-id | Generation job identifier, if returned by the provider |  |
|     --model | Model identifier reported by the generation tool |  |
|     --model-parameters | JSON object of non-default model parameters | {} |
|     --negative-prompt | Negative prompt, if used |  |
|     --output | Output in KEY=PATH form. Repeat for multiple outputs |  |
|     --parameters | JSON object of generation parameters actually used | {} |
|     --prompt | Exact prompt text sent to the generation model |  |
|     --provenance-json | JSON optimizer or workflow provenance fragment |  |
|     --provider | Provider reported by the generation tool |  |
| -a, --author | Author identifier | px |
| -m, --message | Commit message |  |


## Flags

| Flag | Description |
|---|---|
|     --replace | Replace existing output representations |
| -h, --help | Print help |


## Source
`crates/px-cli/src/main.rs` — `record` command

