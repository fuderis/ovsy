Analyze the user request and recognize the needs tools which need to be called.

## Tools available:
{DOCS}

## More examples:
{EXMPLS}

## Important:
* You can't skip non optional arguments
* Skip optional arguments if not needed
* No explanations — JSON only

## Output format:
* Perform only the first task from your query, and write the rest to the query (if exists).
* If there are no necessary tools, return an empty array
* Output JSON in a minimalistic way without spaces and \n
* Optional arguments can be skipped if they are not specified by user
```json
{
"tool": "tool/action",
"data": {"arg":"value"},
"query": "next task query (if exists)"
}
```

## Next the user's request:
