// 定义日志级别类型
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

// 日志工具类
export class Logger {
  private name: string;

  constructor(name: string) {
    this.name = name;
  }

  // 生成带名称前缀的日志消息
  private formatMessage(message: string, args?: any[]): string {
    const prefix = `[${this.name}]`;
    if (args && args.length > 0) {
      return `${prefix} ${message} ${JSON.stringify(args, null, 2)}`;
    }
    return `${prefix} ${message}`;
  }

  // Trace 级别的日志
  trace(message: string, ...args: any[]): void {
    const formattedMessage = this.formatMessage(message, args);
    console.trace(formattedMessage);
  }

  // Debug 级别的日志
  debug(message: string, ...args: any[]): void {
    const formattedMessage = this.formatMessage(message, args);
    console.debug(formattedMessage);
  }

  // Info 级别的日志
  info(message: string, ...args: any[]): void {
    const formattedMessage = this.formatMessage(message, args);
    console.info(formattedMessage);
  }

  // Warn 级别的日志
  warn(message: string, ...args: any[]): void {
    const formattedMessage = this.formatMessage(message, args);
    console.warn(formattedMessage);
  }

  // Error 级别的日志
  error(message: string, ...args: any[]): void {
    const formattedMessage = this.formatMessage(message, args);
    console.error(formattedMessage);
  }
}

// 创建默认的日志实例
export const defaultLogger = new Logger('App');

// 导出便捷的日志方法
export const logTrace = (message: string, ...args: any[]): void => defaultLogger.trace(message, ...args);
export const logDebug = (message: string, ...args: any[]): void => defaultLogger.debug(message, ...args);
export const logInfo = (message: string, ...args: any[]): void => defaultLogger.info(message, ...args);
export const logWarn = (message: string, ...args: any[]): void => defaultLogger.warn(message, ...args);
export const logError = (message: string, ...args: any[]): void => defaultLogger.error(message, ...args);
