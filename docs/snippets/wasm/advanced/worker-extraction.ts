class ExtractionWorker {
  private workerPool: Worker[] = [];
  private taskQueue: Array<{
    bytes: Uint8Array;
    mimeType: string;
    resolve: (value: any) => void;
    reject: (error: any) => void;
  }> = [];

  constructor(workerCount?: number) {
    const count = workerCount ?? (typeof navigator !== 'undefined' ? (navigator.hardwareConcurrency ?? 2) : 2);
    for (let i = 0; i < count; i++) {
      const worker = new Worker("extraction-worker.js");
      worker.onmessage = (e) => this.handleWorkerMessage(worker, e.data);
      worker.onerror = (e) => this.handleWorkerError(worker, e);
      this.workerPool.push(worker);
    }
  }

  async extract(bytes: Uint8Array, mimeType: string) {
    return new Promise((resolve, reject) => {
      this.taskQueue.push({ bytes, mimeType, resolve, reject });
      this.processTasks();
    });
  }

  private processTasks() {
    const availableWorker = this.workerPool[0];
    const task = this.taskQueue.shift();

    if (!task || !availableWorker) return;

    availableWorker.postMessage({ bytes: task.bytes, mimeType: task.mimeType });
    this.workerPool.push(this.workerPool.shift()!);
  }

  private handleWorkerMessage(worker: Worker, result: any) {
    const task = this.taskQueue.shift();
    if (task) {
      task.resolve(result);
      this.processTasks();
    }
  }

  private handleWorkerError(worker: Worker, error: any) {
    const task = this.taskQueue.shift();
    if (task) {
      task.reject(error);
      this.processTasks();
    }
  }

  terminate() {
    this.workerPool.forEach((w) => w.terminate());
  }
}
