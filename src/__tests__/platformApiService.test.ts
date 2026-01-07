import { describe, expect, it, vi, beforeEach } from 'vitest';
import { PlatformApiService } from '../platforms/common/platformApiService';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// 模拟Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('PlatformApiService', () => {
  let platformApiService: PlatformApiService;
  let mockInvoke: vi.Mock;
  let mockListen: vi.Mock;

  beforeEach(() => {
    // 清除所有模拟调用
    vi.clearAllMocks();
    
    // 获取模拟函数
    mockInvoke = invoke as vi.Mock;
    mockListen = listen as vi.Mock;
    
    // 重置单例
    // @ts-ignore - 访问私有静态属性
    PlatformApiService.instance = undefined;
    
    // 获取新实例
    platformApiService = PlatformApiService.getInstance();
  });

  describe('单例模式', () => {
    it('应该返回相同的实例', () => {
      const instance1 = PlatformApiService.getInstance();
      const instance2 = PlatformApiService.getInstance();
      expect(instance1).toBe(instance2);
    });
  });

  describe('平台管理', () => {
    it('应该能够设置和获取当前平台', () => {
      platformApiService.setCurrentPlatform('douyin');
      expect(platformApiService.getCurrentPlatform()).toBe('douyin');
      
      platformApiService.setCurrentPlatform('bilibili');
      expect(platformApiService.getCurrentPlatform()).toBe('bilibili');
    });
  });

  describe('直播列表获取', () => {
    it('应该调用正确的Tauri命令获取直播列表', async () => {
      // 设置模拟返回值
      const mockLiveList = {
        items: [],
        has_more: false
      };
      mockInvoke.mockResolvedValue(mockLiveList);
      
      // 调用方法
      const result = await platformApiService.fetchLiveList('1', 1, 10, 'hot', 'douyu');
      
      // 验证结果
      expect(result).toEqual(mockLiveList);
      expect(mockInvoke).toHaveBeenCalledWith('fetch_live_list', {
        platform: 'douyu',
        category_id: '1',
        page: 1,
        page_size: 10
      });
    });
  });

  describe('直播间信息获取', () => {
    it('应该调用正确的Tauri命令获取直播间信息', async () => {
      // 设置模拟返回值
      const mockRoomInfo = {
        room_id: '123',
        title: '测试直播间',
        streamer_name: '测试主播',
        streamer_id: '456',
        live_status: true,
        live_status_detail: 'LIVE'
      };
      mockInvoke.mockResolvedValue(mockRoomInfo);
      
      // 调用方法
      const result = await platformApiService.fetchRoomInfo('123', 'bilibili');
      
      // 验证结果
      expect(result).toEqual(mockRoomInfo);
      expect(mockInvoke).toHaveBeenCalledWith('fetch_room_info', {
        platform: 'bilibili',
        roomId: '123'
      });
    });
  });

  describe('直播流URL获取', () => {
    it('应该调用正确的Tauri命令获取直播流URL', async () => {
      // 设置模拟返回值
      const mockStreamUrl = {
        primary_url: 'https://example.com/stream',
        available_streams: []
      };
      mockInvoke.mockResolvedValue(mockStreamUrl);
      
      // 调用方法
      const result = await platformApiService.getStreamUrl('123', 'hd', 'douyin');
      
      // 验证结果
      expect(result).toEqual(mockStreamUrl);
      expect(mockInvoke).toHaveBeenCalledWith('get_stream_url', {
        platform: 'douyin',
        roomId: '123',
        quality: 'hd'
      });
    });
  });

  describe('主播信息获取', () => {
    it('应该调用正确的Tauri命令获取主播信息', async () => {
      // 设置模拟返回值
      const mockStreamerInfo = {
        streamer_id: '456',
        name: '测试主播',
        live_status: true,
        live_status_detail: 'LIVE'
      };
      mockInvoke.mockResolvedValue(mockStreamerInfo);
      
      // 调用方法
      const result = await platformApiService.fetchStreamerInfo('456', 'huya');
      
      // 验证结果
      expect(result).toEqual(mockStreamerInfo);
      expect(mockInvoke).toHaveBeenCalledWith('fetch_streamer_info', {
        platform: 'huya',
        streamer_id: '456'
      });
    });
  });

  describe('搜索直播间', () => {
    it('应该调用正确的Tauri命令搜索直播间', async () => {
      // 设置模拟返回值
      const mockSearchResult = {
        items: [],
        has_more: false
      };
      mockInvoke.mockResolvedValue(mockSearchResult);
      
      // 调用方法
      const result = await platformApiService.searchRooms('test', 1, 10, 'douyu');
      
      // 验证结果
      expect(result).toEqual(mockSearchResult);
      expect(mockInvoke).toHaveBeenCalledWith('search_rooms', {
        platform: 'douyu',
        keyword: 'test',
        page: 1,
        page_size: 10
      });
    });
  });

  describe('消息监听', () => {
    it('应该调用正确的Tauri命令启动消息监听', async () => {
      // 设置模拟返回值
      mockInvoke.mockResolvedValue(undefined);
      
      // 调用方法
      await platformApiService.startMessageListener('123', 'bilibili');
      
      // 验证结果
      expect(mockInvoke).toHaveBeenCalledWith('start_message_listener', {
        platform: 'bilibili',
        roomId: '123'
      });
    });

    it('应该调用正确的Tauri命令停止消息监听', async () => {
      // 设置模拟返回值
      mockInvoke.mockResolvedValue(undefined);
      
      // 调用方法
      await platformApiService.stopMessageListener('123', 'douyin');
      
      // 验证结果
      expect(mockInvoke).toHaveBeenCalledWith('stop_message_listener', {
        platform: 'douyin',
        roomId: '123'
      });
    });

    it('应该调用正确的Tauri命令订阅消息事件', async () => {
      // 设置模拟返回值
      const mockUnlisten = vi.fn();
      mockListen.mockResolvedValue(mockUnlisten);
      
      // 调用方法
      const callback = vi.fn();
      const result = await platformApiService.subscribeToMessages(callback, '123');
      
      // 验证结果
      expect(mockListen).toHaveBeenCalledWith('message', expect.any(Function));
      expect(result).toBe(mockUnlisten);
    });
  });

  describe('检查直播间状态', () => {
    it('应该正确检查直播间状态', async () => {
      // 设置模拟返回值
      const mockRoomInfo = {
        room_id: '123',
        title: '测试直播间',
        streamer_name: '测试主播',
        streamer_id: '456',
        live_status: true,
        live_status_detail: 'LIVE'
      };
      mockInvoke.mockResolvedValue(mockRoomInfo);
      
      // 调用方法
      const result = await platformApiService.checkRoomStatus('123', 'bilibili');
      
      // 验证结果
      expect(result).toBe(true);
      expect(mockInvoke).toHaveBeenCalledWith('fetch_room_info', {
        platform: 'bilibili',
        roomId: '123'
      });
    });

    it('应该在获取失败时返回false', async () => {
      // 设置模拟返回值为错误
      mockInvoke.mockRejectedValue(new Error('Failed to fetch room info'));
      
      // 调用方法
      const result = await platformApiService.checkRoomStatus('123', 'bilibili');
      
      // 验证结果
      expect(result).toBe(false);
    });
  });

  describe('分类相关方法', () => {
    it('应该调用正确的Tauri命令获取分类列表', async () => {
      // 设置模拟返回值
      const mockCategories = [
        { id: '1', name: '游戏', href: '/game' },
        { id: '2', name: '娱乐', href: '/entertainment' }
      ];
      mockInvoke.mockResolvedValue(mockCategories);
      
      // 调用方法
      const result = await platformApiService.fetchCategories('0', 'douyu');
      
      // 验证结果
      expect(result).toEqual(mockCategories);
      expect(mockInvoke).toHaveBeenCalledWith('fetch_categories', {
        platform: 'douyu'
      });
    });

    it('应该在获取失败时返回空数组', async () => {
      // 设置模拟返回值为错误
      mockInvoke.mockRejectedValue(new Error('Failed to fetch categories'));
      
      // 调用方法
      const result = await platformApiService.fetchCategories('0', 'douyu');
      
      // 验证结果
      expect(result).toEqual([]);
    });
  });

  describe('认证相关方法', () => {
    it('应该调用正确的Tauri命令获取认证URL', async () => {
      // 设置模拟返回值
      const mockAuthUrl = 'https://example.com/auth';
      mockInvoke.mockResolvedValue(mockAuthUrl);
      
      // 调用方法
      const result = await platformApiService.getAuthUrl('bilibili');
      
      // 验证结果
      expect(result).toBe(mockAuthUrl);
      expect(mockInvoke).toHaveBeenCalledWith('get_auth_url', {
        platform: 'bilibili'
      });
    });

    it('应该调用正确的Tauri命令使用认证码登录', async () => {
      // 设置模拟返回值
      const mockUserInfo = {
        id: '123',
        name: 'test_user',
        avatar: 'https://example.com/avatar.jpg'
      };
      mockInvoke.mockResolvedValue(mockUserInfo);
      
      // 调用方法
      const result = await platformApiService.loginWithCode('test_code', 'douyin');
      
      // 验证结果
      expect(result).toEqual(mockUserInfo);
      expect(mockInvoke).toHaveBeenCalledWith('login_with_code', {
        platform: 'douyin',
        code: 'test_code'
      });
    });

    it('应该调用正确的Tauri命令刷新认证令牌', async () => {
      // 设置模拟返回值
      const mockUserInfo = {
        id: '123',
        name: 'test_user',
        avatar: 'https://example.com/avatar.jpg'
      };
      mockInvoke.mockResolvedValue(mockUserInfo);
      
      // 调用方法
      const result = await platformApiService.refreshToken('huya');
      
      // 验证结果
      expect(result).toEqual(mockUserInfo);
      expect(mockInvoke).toHaveBeenCalledWith('refresh_token', {
        platform: 'huya'
      });
    });

    it('应该调用正确的Tauri命令获取当前用户信息', async () => {
      // 设置模拟返回值
      const mockUserInfo = {
        id: '123',
        name: 'test_user',
        avatar: 'https://example.com/avatar.jpg'
      };
      mockInvoke.mockResolvedValue(mockUserInfo);
      
      // 调用方法
      const result = await platformApiService.getCurrentUser('douyu');
      
      // 验证结果
      expect(result).toEqual(mockUserInfo);
      expect(mockInvoke).toHaveBeenCalledWith('get_current_user', {
        platform: 'douyu'
      });
    });
  });
});
