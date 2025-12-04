import cytoscape from 'cytoscape';
import dagre from 'cytoscape-dagre';

cytoscape.use(dagre);

export function createCytoscape(options) {
  return cytoscape(options);
}
