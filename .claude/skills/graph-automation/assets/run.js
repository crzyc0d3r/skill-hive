export const meta = {
  name: 'graph-automation-wave',
  description: 'Run one wave of ready model nodes from a graph-automation bundle as parallel bounded subagents',
  whenToUse: 'Only from the graph-automation skill, after graphctl ready; pass {nodes:[{id, brief}]} as args',
  phases: [{ title: 'Nodes', detail: 'one subagent per ready model node, each given only its brief' }],
}

// The orchestrator (main session) stays in charge of readiness, gating and
// correction edges because those need the filesystem (graphctl). This script
// only fans out the ready model nodes of ONE wave and returns their reports,
// validated against the schema every brief asks for.
const REPORT = {
  type: 'object',
  properties: {
    node: { type: 'string' },
    output_path: { type: 'string' },
    verdict: { type: 'string', enum: ['pass', 'fail'] },
    evidence: { type: 'string' },
    notes: { type: 'string' },
  },
  required: ['node', 'output_path', 'verdict', 'evidence'],
}

const nodes = (args && Array.isArray(args.nodes)) ? args.nodes : []
if (!nodes.length) return { reports: [], note: 'args.nodes was empty: pass [{id, brief}] from graphctl dispatch' }

phase('Nodes')
log(`dispatching ${nodes.length} ready node(s): ${nodes.map(n => n.id).join(', ')}`)

const reports = await pipeline(
  nodes,
  (n) => agent(n.brief, { label: n.id, phase: 'Nodes', schema: REPORT, agentType: 'general-purpose' })
    .then(r => r ? { ...r, node: r.node || n.id } : { node: n.id, output_path: n.output || '', verdict: 'fail', evidence: 'agent returned nothing (skipped or terminal error)' })
)

return { reports: reports.filter(Boolean) }
