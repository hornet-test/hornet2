import { create } from 'zustand';

export interface ProjectInfo {
  name: string;
  title: string;
  description?: string;
  workflow_count: number;
  arazzo_path: string;
  openapi_files: string[];
}

interface ProjectState {
  projects: ProjectInfo[];
  currentProject: string | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  loadProjects: () => Promise<void>;
  selectProject: (name: string) => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  currentProject: null,
  isLoading: false,
  error: null,

  loadProjects: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await fetch('/api/projects');
      if (!response.ok) {
        throw new Error(`Failed to load projects: ${response.statusText}`);
      }
      const data = (await response.json()) as { projects: ProjectInfo[] };
      set({ projects: data.projects, isLoading: false });

      // Auto-select first project if none selected and we have projects
      // But maybe we want to persist selection in URL or localStorage?
      // For now, let's auto-select first one to keep simple behavior expected by user.
      const { currentProject } = get();
      if (!currentProject && data.projects.length > 0) {
        set({ currentProject: data.projects[0].name });
      }
    } catch (error) {
      set({
        error: (error as Error).message,
        isLoading: false,
      });
    }
  },

  selectProject: (name: string) => {
    set({ currentProject: name });
  },
}));
