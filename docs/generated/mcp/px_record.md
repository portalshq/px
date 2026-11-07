---
generated: "true"
generator: px-docgen
version: 0.10.0
source: mcp
---


# px_record
Record generated outputs, their exact prompt, and provenance together


## Parameters

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| author | string | No | px | Author identifier |
| branch | string | Yes |  | Branch currently checked out in the repository |
| input\_refs | string | No | [] | JSON array of input references with URI, key, hash, and commit |
| job\_id | string | No |  | Generation job identifier, if returned by the provider |
| message | string | No |  | Commit message |
| model | string | No |  | Model identifier reported by the generation tool |
| model\_parameters | string | No | {} | JSON object of non-default model parameters |
| negative\_prompt | string | No |  | Negative prompt, if used |
| output | string or string[] | Yes |  | Output in KEY=PATH form. Repeat for multiple outputs |
| parameters | string | No | {} | JSON object of generation parameters actually used |
| prompt | string | Yes |  | Exact prompt text sent to the generation model |
| provenance\_json | string | No |  | JSON optimizer or workflow provenance fragment |
| provider | string | No |  | Provider reported by the generation tool |
| replace | boolean | No | false | Replace existing output representations |
| uri | string | Yes |  | PX entity URI |

