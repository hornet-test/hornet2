import { useState } from 'react';
import { VisualizationPage } from './pages/VisualizationPage';
import { EditorPage } from './pages/EditorPage';

type PageType = 'visualization' | 'editor';

export default function App() {
  const [currentPage, setCurrentPage] = useState<PageType>('visualization');

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-content">
          <div className="header-title">
            <h1>Hornet2</h1>
            <p className="subtitle">Document-driven API testing tool</p>
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
        {currentPage === 'visualization' && <VisualizationPage />}
        {currentPage === 'editor' && <EditorPage />}
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
        }
      `}</style>
    </div>
  );
}
