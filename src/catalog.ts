export const CATEGORIES = ["语言与运行时", "开发基础", "数据库与服务", "开发工具"] as const;
export type Category = (typeof CATEGORIES)[number];
export type Stage = "first" | "planned" | "later";

export interface Software {
  id: string;
  name: string;
  category: Category;
  description: string;
  /** Larger means earlier in the not-installed grid. */
  popularity: number;
  accent: string;
  /** Planned installation mechanism, shown honestly in phase 1. */
  source: string;
  stage: Stage;
}

export const STAGE_LABEL: Record<Stage, string> = {
  first: "首批支持",
  planned: "后续支持",
  later: "再后续支持"
};

export const SOFTWARE: Software[] = [
  {
    id: "python",
    name: "Python",
    category: "语言与运行时",
    description: "数据分析、自动化与 AI 开发最常用的运行时。",
    popularity: 100,
    accent: "#4B8BBE",
    source: "官方发行版 / Homebrew",
    stage: "planned"
  },
  {
    id: "node",
    name: "Node.js",
    category: "语言与运行时",
    description: "JavaScript 与前端工具链的运行时。",
    popularity: 98,
    accent: "#5FA04E",
    source: "DevHub 受管官方发行物",
    stage: "first"
  },
  {
    id: "git",
    name: "Git",
    category: "开发基础",
    description: "版本控制，几乎每个项目都需要的开发基础工具。",
    popularity: 96,
    accent: "#F05032",
    source: "Homebrew formula",
    stage: "first"
  },
  {
    id: "vscode",
    name: "VS Code",
    category: "开发工具",
    description: "轻量、扩展丰富的代码编辑器。",
    popularity: 92,
    accent: "#007ACC",
    source: "Homebrew cask（后续）",
    stage: "later"
  },
  {
    id: "jdk",
    name: "JDK",
    category: "语言与运行时",
    description: "Java 开发与运行环境，Maven、Gradle 的基础。",
    popularity: 90,
    accent: "#E76F00",
    source: "Homebrew openjdk / 官方发行版",
    stage: "planned"
  },
  {
    id: "go",
    name: "Go",
    category: "语言与运行时",
    description: "编译型语言与标准工具链，适合服务端开发。",
    popularity: 86,
    accent: "#00ADD8",
    source: "官方发行版 / Homebrew",
    stage: "planned"
  },
  {
    id: "docker",
    name: "Docker",
    category: "数据库与服务",
    description: "用容器隔离和运行开发服务。",
    popularity: 80,
    accent: "#2496ED",
    source: "暂缓（需要管理员权限或虚拟机）",
    stage: "later"
  },
  {
    id: "mysql",
    name: "MySQL",
    category: "数据库与服务",
    description: "常见的关系型数据库。",
    popularity: 78,
    accent: "#00758F",
    source: "Homebrew + 服务（后续）",
    stage: "later"
  },
  {
    id: "postgresql",
    name: "PostgreSQL",
    category: "数据库与服务",
    description: "功能完整的关系型数据库。",
    popularity: 76,
    accent: "#336791",
    source: "Homebrew + 服务（后续）",
    stage: "later"
  },
  {
    id: "maven",
    name: "Maven",
    category: "开发基础",
    description: "Java 项目的构建与依赖管理工具。",
    popularity: 74,
    accent: "#C71A36",
    source: "Homebrew formula",
    stage: "planned"
  },
  {
    id: "gradle",
    name: "Gradle",
    category: "开发基础",
    description: "多语言项目构建工具，Android 开发常用。",
    popularity: 72,
    accent: "#02303A",
    source: "Homebrew formula",
    stage: "planned"
  },
  {
    id: "redis",
    name: "Redis",
    category: "数据库与服务",
    description: "内存数据结构与缓存服务。",
    popularity: 68,
    accent: "#DC382D",
    source: "Homebrew + 服务（后续）",
    stage: "later"
  },
  {
    id: "rust",
    name: "Rust",
    category: "语言与运行时",
    description: "注重内存安全与性能的系统编程语言。",
    popularity: 64,
    accent: "#CE422B",
    source: "rustup（后续）",
    stage: "later"
  },
  {
    id: "intellij",
    name: "IntelliJ IDEA",
    category: "开发工具",
    description: "面向 JVM 的重量级集成开发环境。",
    popularity: 60,
    accent: "#FE315D",
    source: "Homebrew cask（后续）",
    stage: "later"
  }
];
