import type { Software } from "../catalog";

interface SoftwareDescription {
  purpose: string;
  uses: readonly string[];
}

// Explain the software itself. Installation policy belongs to the action confirmation.
const DESCRIPTIONS: Record<string, SoftwareDescription> = {
  python: {
    purpose: "通用编程语言，拥有丰富的数据处理、自动化和机器学习生态。",
    uses: ["编写脚本，批量处理文件、接口和数据。", "运行 Python 项目，配合虚拟环境隔离各项目依赖。"]
  },
  node: {
    purpose: "在浏览器之外运行 JavaScript 的运行时，也是 npm 开发工具的运行基础。",
    uses: ["开发 Web 服务、命令行工具和自动化脚本。", "运行 Vite、React 等前端项目的开发与构建工具。"]
  },
  git: {
    purpose: "分布式版本控制工具，用提交记录文件变化，用分支隔离不同开发任务。",
    uses: ["查看修改、恢复历史版本和合并代码。", "与 GitHub 等代码托管平台同步并协作开发。"]
  },
  vscode: {
    purpose: "可通过扩展适配不同语言的代码编辑器，集成终端、调试和 Git 操作。",
    uses: ["编辑项目代码，在同一窗口运行与调试。", "按项目添加语言支持、格式化和代码检查扩展。"]
  },
  jdk: {
    purpose: "Java 开发工具包，包含 Java 运行环境、javac 编译器及诊断工具。",
    uses: ["编译、运行和调试 Java 程序。", "为 Maven、Gradle 等 JVM 项目提供 Java 工具链。"]
  },
  go: {
    purpose: "编译型语言及工具链，内置依赖管理、格式化、测试和并发编程支持。",
    uses: ["开发网络服务、API 和命令行工具。", "通过 go build 构建程序，通过 go test 运行测试。"]
  },
  docker: {
    purpose: "用镜像和容器封装应用及其运行依赖，在不同机器上复用一致的环境。",
    uses: ["隔离运行数据库、缓存和其他开发服务。", "构建应用镜像，使用 Compose 编排多个服务。"]
  },
  mysql: {
    purpose: "关系型数据库，使用 SQL 定义表、查询数据并处理事务。",
    uses: ["存储业务记录，构建需要关联查询的应用。", "在本地开发和测试依赖 MySQL 的项目。"]
  },
  postgresql: {
    purpose: "开源关系型数据库，支持事务、复杂 SQL 查询及 JSON 数据处理。",
    uses: ["作为业务系统的持久化数据库。", "通过扩展支持地理空间等专门的数据场景。"]
  },
  maven: {
    purpose: "以 pom.xml 描述依赖和构建流程的 Java 项目管理工具。",
    uses: ["统一下载依赖、编译、测试和打包流程。", "构建多模块项目并发布 Java 制品。"]
  },
  gradle: {
    purpose: "使用 Kotlin 或 Groovy 脚本定义任务的构建工具，支持增量构建和依赖管理。",
    uses: ["构建 Java、Kotlin 和 Android 项目。", "组合测试、打包与其他自定义构建任务。"]
  },
  redis: {
    purpose: "内存数据存储，提供字符串、哈希、列表、集合等数据结构。",
    uses: ["实现缓存、计数器、排行榜和会话存储。", "通过发布订阅或 Streams 处理消息。"]
  },
  rust: {
    purpose: "强调内存安全的编译型语言，适合需要性能和精细资源控制的程序。",
    uses: ["开发命令行工具、系统组件和服务端程序。", "通过 Cargo 管理依赖、构建项目和运行测试。"]
  },
  intellij: {
    purpose: "面向 Java、Kotlin 等 JVM 项目的集成开发环境，提供代码分析、重构和调试。",
    uses: ["浏览和修改大型 JVM 项目。", "在编辑器内导入 Maven、Gradle 项目并运行测试。"]
  },
  memcached: {
    purpose: "轻量的内存键值缓存服务，用来减少应用反复访问数据库或执行昂贵计算。",
    uses: ["缓存查询结果、页面片段等可重新生成的数据。", "应用通过客户端读写缓存；数据会随过期、淘汰或服务重启消失。"]
  }
};

const PACKAGE_IDS: Record<string, string> = {
  "visual-studio-code": "vscode",
  openjdk: "jdk",
  "postgresql@17": "postgresql",
  "intellij-idea": "intellij"
};

export function softwareDescription(item: Pick<Software, "id" | "package">): SoftwareDescription | undefined {
  return DESCRIPTIONS[item.id] ?? DESCRIPTIONS[PACKAGE_IDS[item.package] ?? item.package];
}
