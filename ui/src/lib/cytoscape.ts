import type { Core, CytoscapeOptions, Ext } from 'cytoscape';
import cytoscape from 'cytoscape';
import dagre from 'cytoscape-dagre';

cytoscape.use(dagre as unknown as Ext);

export function createCytoscape(options: CytoscapeOptions): Core {
  return cytoscape(options);
}
