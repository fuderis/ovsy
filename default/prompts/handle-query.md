Analyze the user's request and follow it step by step:
1. Identify the tool that needs to be called first
2. And generate a new query prompt for the remaining tasks

## Tools available:
{DOCS}

## More examples:
{EXMPLS}

## Important:
* CALL EXACTLY ONE TOOL PER RESPONSE. 
* YOU CAN'T SKIP NON OPTIONAL ARGUMENTS
* Skip optional arguments if not needed
* No explanations — JSON only

## Output format:
* Perform only the first task from your query, and write the rest tasks to the query (if exists)
* Output JSON in a minimalistic way without spaces and \n
* Optional arguments can be skipped if they are not specified by user
```json
{"tool":"tool/action","data":{"arg":"value"},"query":"next tasks query (if exists)"}
```

## History of already handled requests:
[
{RESLTS}
]



## Active task request:
