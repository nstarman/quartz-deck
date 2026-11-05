# HTTP contract additions (007)

All routes keep spec 004's rules: loopback `Host` only. Writes need loopback
`Origin` + `X-QD-Token` and JSON.

| Route | Auth | Body → Response |
|---|---|---|
| `GET /api/settings` | local | → `{values: {key: value}, defs: [{key, label, description, type, values?, default}], notices: [string]}` |
| `POST /api/settings` | token | `{key: value, …}` (partial) → `{values}`; 400 `E_INVALID_SETTING` naming the key; `{"reset": true}` resets |
| `GET /ui/actions.js` | local | `export default [...]` (the action list) |
| `POST /api/op` `{"op": "paste", "args": {"after"?, "before"?, "md"}}` | as other ops | → op result; one journal entry |

SSE: `settings` event `{values}` after any change (from this server, the CLI
or another server writing the file).
