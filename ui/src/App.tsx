import { useState, useEffect } from 'react';
import { VisualizationPage } from './pages/VisualizationPage';
import { EditorPage } from './pages/EditorPage';
import { useProjectStore } from './stores/projectStore';

type PageType = 'visualization' | 'editor';

export default function App() {
  const [currentPage, setCurrentPage] = useState<PageType>('visualization');
  const {
    projects,
    currentProject,
    selectProject,
    loadProjects,
    isLoading: isProjectLoading,
    error: projectError,
  } = useProjectStore();

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-content">
          <div className="header-left">
            <div className="header-title">
              <h1>Hornet2</h1>
              <p className="subtitle">Document-driven API testing tool</p>
            </div>

            <div className="project-selector">
              <label htmlFor="project-select">Project:</label>
              <select
                id="project-select"
                value={currentProject || ''}
                onChange={(e) => selectProject(e.target.value)}
                disabled={isProjectLoading}
              >
                {!currentProject && <option value="">Select Project...</option>}
                {projects.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.title || p.name}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <nav className="nav-tabs">
            <button
              className={`nav-tab ${currentPage === 'visualization' ? 'active' : ''}`}
              onClick={() => setCurrentPage('visualization')}
            >
              Visualization
            </button>
            <button
              className={`nav-tab ${currentPage === 'editor' ? 'active' : ''}`}
              onClick={() => setCurrentPage('editor')}
            >
              Editor
            </button>
          </nav>
        </div>
      </header>

      <main className="app-main">
        {projectError && <div className="error-banner">{projectError}</div>}

        {!currentProject ? (
          <div className="no-project">
            {isProjectLoading ? 'Loading projects...' : 'Please select a project.'}
          </div>
        ) : (
          <>
            {currentPage === 'visualization' && <VisualizationPage />}
            {currentPage === 'editor' && <EditorPage />}
          </>
        )}
      </main>

      <style>{`
        * {
          box-sizing: border-box;
        }

        html,
        body,
        #root,
        .app {
          margin: 0;
          padding: 0;
          height: 100%;
          width: 100%;
          overflow: hidden;
        }

        body {
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
            'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
            sans-serif;
          -webkit-font-smoothing: antialiased;
          -moz-osx-font-smoothing: grayscale;
        }

        code {
          font-family: source-code-pro, Menlo, Monaco, Consolas, 'Courier New', monospace;
        }

        .app {
          display: flex;
          flex-direction: column;
        }

        .app-header {
          background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
          color: white;
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
          flex-shrink: 0;
        }

        .header-content {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1rem 2rem;
          max-width: 100%;
        }

        .header-left {
          display: flex;
          align-items: center;
          gap: 2rem;
        }

        .header-title h1 {
          margin: 0;
          font-size: 1.75rem;
          font-weight: 700;
        }

        .header-title .subtitle {
          margin: 0.25rem 0 0 0;
          font-size: 0.9rem;
          opacity: 0.9;
        }

        .project-selector {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          background: rgba(255, 255, 255, 0.1);
          padding: 0.5rem 1rem;
          border-radius: 6px;
        }

        .project-selector label {
          font-weight: 500;
          font-size: 0.9rem;
        }

        .project-selector select {
          background: rgba(255, 255, 255, 0.2);
          border: 1px solid rgba(255, 255, 255, 0.3);
          color: white;
          padding: 0.25rem 0.5rem;
          border-radius: 4px;
          outline: none;
          font-size: 0.9rem;
          min-width: 150px;
        }
        
        .project-selector select option {
          background: white;
          color: black;
        }

        .nav-tabs {
          display: flex;
          gap: 0.5rem;
        }

        .nav-tab {
          padding: 0.75rem 1.5rem;
          background: rgba(255, 255, 255, 0.1);
          color: white;
          border: 1px solid rgba(255, 255, 255, 0.2);
          border-radius: 6px;
          font-size: 0.95rem;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s;
        }

        .nav-tab:hover {
          background: rgba(255, 255, 255, 0.2);
        }

        .nav-tab.active {
          background: white;
          color: #667eea;
          border-color: white;
        }

        .app-main {
          flex: 1;
          overflow: hidden;
          display: flex;
          flex-direction: column;
        }
        
        .error-banner {
          background: #dc3545;
          color: white;
          padding: 1rem;
          text-align: center;
        }
        
        .no-project {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          color: #6c757d;
          font-size: 1.2rem;
        }
      `}</style>
    </div>
  );
}
