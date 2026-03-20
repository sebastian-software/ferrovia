# ADR 0008: Tree Rewrite Mutation Rules

## Status

Accepted

## Context

The deterministic plugin wave mostly operated on local node removal and attribute sorting. The next wave introduces structure-changing plugins such as `removeEmptyContainers`, `moveGroupAttrsToElems`, `moveElemsAttrsToGroup`, and `collapseGroups`.

These plugins do not just delete nodes. They reparent children, merge attributes, and depend on deterministic output after tree mutation. Without explicit rules, small helper changes could silently alter serializer order, break previously green fixtures, or make plugins interfere with each other.

## Decision

Tree rewrite plugins must follow these mutation rules:

- Child lists are rewritten explicitly and deterministically. Reparenting must use arena-safe operations that leave `parent`, `first_child`, `last_child`, and `next_sibling` coherent after every plugin run.
- Nodes that move under a new parent must be detached from the old parent before reordering the target child list.
- Attribute merges must preserve deterministic ordering. A plugin may only change attribute order when that order is itself part of the plugin semantics or a separately tested normalization step.
- Plugins in this wave must not introduce silent serializer changes. If a tree rewrite needs a serialization difference, that change must be justified by differential tests.
- Each tree rewrite plugin is implemented and committed in isolation, with targeted structure tests in addition to the existing regression gates.

## Consequences

- The current arena helpers remain intentionally small, but their behavior becomes part of the compatibility contract for structure-changing plugins.
- Plugin authors must prefer explicit child-list rewrites over ad hoc sibling-pointer mutation spread across multiple branches.
- Future tree rewrite plugins can build on the same constraints without reopening basic mutation semantics.
