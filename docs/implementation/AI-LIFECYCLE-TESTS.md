# AI lifecycle test seam

`settle_proposal` is the one production path that waits for an AI proposal,
the 180-second deadline, or cancellation. It owns only lifecycle settlement:
`working` becomes `preview`, `failed`, or `cancelled`; cancelled and failed
jobs retain neither context nor plan.

The focused bridge tests use pending futures and `Notify`, not an external CLI,
provider, filesystem workspace, or network server. They prove cancellation,
short deterministic timeout, a late successful future after cancellation, and
the no-rollback rule while an apply is already confirmed.

They do not prove provider response quality, subprocess-tree termination, or
external CLI cancellation behavior. Those remain outside this seam and require
separate native/provider evidence.
