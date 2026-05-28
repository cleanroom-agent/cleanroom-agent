// A simple Todo App — test fixture for Cleanroom Agent.

export interface Todo {
  id: string;
  title: string;
  completed: boolean;
  createdAt: Date;
  updatedAt?: Date;
}

export type TodoStatus = 'active' | 'completed' | 'deleted';

export interface CreateTodoInput {
  title: string;
}

export interface UpdateTodoInput {
  title?: string;
  completed?: boolean;
}

export interface TodoFilter {
  status?: TodoStatus;
  search?: string;
}

export class TodoService {
  private todos: Map<string, Todo> = new Map();

  async create(input: CreateTodoInput): Promise<Todo> {
    const todo: Todo = {
      id: crypto.randomUUID(),
      title: input.title,
      completed: false,
      createdAt: new Date(),
    };
    this.todos.set(todo.id, todo);
    return todo;
  }

  async getById(id: string): Promise<Todo | null> {
    return this.todos.get(id) ?? null;
  }

  async update(id: string, input: UpdateTodoInput): Promise<Todo | null> {
    const existing = this.todos.get(id);
    if (!existing) return null;
    const updated: Todo = {
      ...existing,
      ...input,
      updatedAt: new Date(),
    };
    this.todos.set(id, updated);
    return updated;
  }

  async delete(id: string): Promise<boolean> {
    return this.todos.delete(id);
  }

  async list(filter?: TodoFilter): Promise<Todo[]> {
    let result = Array.from(this.todos.values());
    if (filter?.status) {
      result = result.filter(t => {
        if (filter.status === 'active') return !t.completed;
        if (filter.status === 'completed') return t.completed;
        return true;
      });
    }
    if (filter?.search) {
      const q = filter.search.toLowerCase();
      result = result.filter(t => t.title.toLowerCase().includes(q));
    }
    return result.sort((a, b) =>
      b.createdAt.getTime() - a.createdAt.getTime()
    );
  }

  async count(): Promise<number> {
    return this.todos.size;
  }
}
