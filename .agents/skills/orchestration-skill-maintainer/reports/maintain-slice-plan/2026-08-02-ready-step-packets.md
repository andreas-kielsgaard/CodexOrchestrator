# Ready Plan Step packets

## Observation and theory

The Slice Plan presentation listed sequence, parallel lanes, gates, and a launch register, but it did not require a concrete startable work package. The observed plans could therefore state one broad implementation step and a later gate without demonstrating which concerns formed independent lanes or what could start together.

The historical Sprint planning model explicitly required concern-to-unit and dependency maps, parallel lanes, shared integration surfaces, a first eligible packet, and final convergence.

## Revision

Every Plan Step now records hard dependencies, preferred ordering, gates, shared surfaces, and compatibility of ownership and work routes. The planner groups all currently eligible independent Steps into the next ready packet and shows entry evidence, parallel lanes, gates, convergence, unlocks, and held work. A one-Step packet requires a concrete dependency or overlap rationale.

## Evaluation

This restores work-package planning while keeping Plan Steps outcome-oriented. It makes serial execution explainable and parallel execution deliberate rather than rewarding either pattern by default.

A fresh test formed A and B as the current ready packet on separate attributable routes, held C and D behind their respective acceptance gates, identified E as convergence after accepted C and D, and kept F behind a reserved decision. Its launch register included every Step, gate, and the ready packet itself.
