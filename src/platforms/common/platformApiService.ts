import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SupportedPlatform } from './types';

// 分类类型定义
export interface Category {
  id: string;
  name: string;
  href: string;
  subcategories?: Category[];
}

// 统一的直播流质量类型
export type UnifiedStreamQuality = 'ultrahd' | 'hd' | 'sd' | 'auto' | string;

// 统一的直播间信息
export interface UnifiedRoomInfo {
  room_id: string;
  title: string;
  streamer_name: string;
  streamer_id: string;
  avatar_url?: string;
  cover_url?: string;
  live_status: boolean;
  viewer_count?: number;
  category_name?: string;
  category_id?: string;
  tags?: string[];
  live_status_detail: 'LIVE' | 'REPLAY' | 'OFFLINE' | 'UNKNOWN';
}

// 统一的直播列表
export interface UnifiedLiveList {
  items: UnifiedRoomInfo[];
  total?: number;
  page?: number;
  page_size?: number;
  has_more: boolean;
}

// 统一的主播信息
export interface UnifiedStreamerInfo {
  streamer_id: string;
  name: string;
  avatar_url?: string;
  bio?: string;
  follower_count?: number;
  live_status: boolean;
  room_id?: string;
  room_title?: string;
  live_status_detail: 'LIVE' | 'REPLAY' | 'OFFLINE' | 'UNKNOWN';
}

// 统一的直播流URL信息
export interface UnifiedStreamUrl {
  primary_url: string;
  upstream_url?: string;
  available_streams: {
    url: string;
    format?: string | null;
    desc?: string | null;
    qn?: number | null;
    protocol?: string | null;
  }[];
  room_info?: UnifiedRoomInfo;
  streamer_info?: UnifiedStreamerInfo;
}

// 用户信息类型
export interface UserInfo {
  id: string;
  name: string;
  avatar?: string;
  level?: number;
  [key: string]: unknown;
}

// 统一的消息类型定义
export interface UnifiedMessage {
  id: string;
  message_type: string;
  room_id: string;
  user: string;
  content: string;
  user_level?: number;
  fans_level?: number;
  fans_club_level?: number;
  badge_name?: string;
  badge_level?: number;
  uid?: string;
  color?: string;
  timestamp: number;
  platform: SupportedPlatform;
  
  // 礼物相关字段
  gift_name?: string;
  gift_count?: number;
  gift_price?: number;
  gift_total?: number;
  
  // 超级弹幕相关字段
  super_chat_price?: number;
  super_chat_duration?: number;
  
  // 连击相关字段
  combo_count?: number;
  combo_user?: string;
  
  // 原始数据
  raw?: any;
  
  // 其他扩展字段
  other?: any;
}

// 导入FollowedStreamer类型
import type { FollowedStreamer } from './types';

// 统一的API服务类
export class PlatformApiService {
  private static instance: PlatformApiService;
  private currentPlatform: SupportedPlatform = 'douyu';
  
  private constructor() {
    // 私有构造函数，防止直接实例化
  }
  
  // 获取单例实例
  public static getInstance(): PlatformApiService {
    if (!PlatformApiService.instance) {
      PlatformApiService.instance = new PlatformApiService();
    }
    return PlatformApiService.instance;
  }
  
  // 设置当前平台
  public setCurrentPlatform(platform: SupportedPlatform): void {
    this.currentPlatform = platform;
  }
  
  // 获取当前平台
  public getCurrentPlatform(): SupportedPlatform {
    return this.currentPlatform;
  }
  
  // 平台类型转换：字符串 -> Platform 枚举
  private getPlatformEnum(platform?: SupportedPlatform): string {
    const targetPlatform = platform || this.currentPlatform;
    return targetPlatform;
  }
  
  // 获取直播列表
  public async fetchLiveList(
    categoryId?: string,
    page?: number,
    pageSize?: number,
    _sort?: string,
    platform?: SupportedPlatform
  ): Promise<UnifiedLiveList> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      // 使用统一的fetch_live_list调用
      const result = await invoke<UnifiedLiveList>('fetch_live_list', {
        platform: this.getPlatformEnum(targetPlatform),
        category_id: categoryId,
        page,
        page_size: pageSize,
      });
      return result;
    } catch (error) {
      console.error('Failed to fetch live list:', error);
      throw error;
    }
  }
  
  // 获取直播间信息
  public async fetchRoomInfo(
    roomId: string,
    platform?: SupportedPlatform
  ): Promise<UnifiedRoomInfo> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      // 使用统一的fetch_room_info调用
      const result = await invoke<UnifiedRoomInfo>('fetch_room_info', {
        platform: this.getPlatformEnum(targetPlatform),
        roomId,
      });
      return result;
    } catch (error) {
      console.error('Failed to fetch room info:', error);
      throw error;
    }
  }
  
  // 获取直播流URL
  public async getStreamUrl(
    roomId: string,
    quality: UnifiedStreamQuality = 'auto',
    platform?: SupportedPlatform
  ): Promise<UnifiedStreamUrl> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      // 使用统一的get_stream_url调用
      return await invoke<UnifiedStreamUrl>('get_stream_url', {
        platform: this.getPlatformEnum(targetPlatform),
        roomId,
        quality: quality as string,
      });
    } catch (error) {
      console.error('Failed to get stream url:', error);
      throw error;
    }
  }
  
  // 获取主播信息
  public async fetchStreamerInfo(
    streamerId: string,
    platform?: SupportedPlatform
  ): Promise<UnifiedStreamerInfo> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      // 使用统一的fetch_streamer_info调用
      const result = await invoke<UnifiedStreamerInfo>('fetch_streamer_info', {
        platform: this.getPlatformEnum(targetPlatform),
        streamer_id: streamerId,
      });
      return result;
    } catch (error) {
      console.error('Failed to fetch streamer info:', error);
      throw error;
    }
  }
  
  // 搜索直播间
  public async searchRooms(
    keyword: string,
    page?: number,
    pageSize?: number,
    platform?: SupportedPlatform
  ): Promise<UnifiedLiveList> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      // 使用统一的search_rooms调用
      const result = await invoke<UnifiedLiveList>('search_rooms', {
        platform: this.getPlatformEnum(targetPlatform),
        keyword,
        page,
        page_size: pageSize,
      });
      return result;
    } catch (error) {
      console.error('Failed to search rooms:', error);
      throw error;
    }
  }
  
  // 订阅消息事件
  public async subscribeToMessages(
    callback: (message: UnifiedMessage) => void,
    roomId?: string
  ): Promise<() => void> {
    try {
      const unlisten = await listen<any>('message', (event) => {
        let messageData = event.payload;
        console.log('Received message:', event);
        // 处理后端发送的消息格式：{ platform, room_id, message } 或直接是 UnifiedMessage
        let processedMessage: any = messageData;
        
        // 如果消息嵌套在 message 字段中，则提取实际消息内容
        if (messageData.message && typeof messageData.message === 'object') {
          processedMessage = messageData.message;
          
          // 添加缺失的平台和房间ID字段
          if (!processedMessage.platform && messageData.platform) {
            processedMessage.platform = messageData.platform;
          }
          
          if (!processedMessage.room_id && messageData.room_id) {
            processedMessage.room_id = messageData.room_id;
          }
        }
        
        // 验证消息格式，确保包含必要字段
        if (processedMessage && typeof processedMessage === 'object' && processedMessage.content) {
          // 检查房间ID过滤条件
          if (!roomId || processedMessage.room_id === roomId) {
            // 将处理后的消息转换为 UnifiedMessage 类型并调用回调
            callback(processedMessage as UnifiedMessage);
          }
        }
      });
      return unlisten;
    } catch (error) {
      console.error('Failed to subscribe to messages:', error);
      throw error;
    }
  }
  
  // 检查直播间状态
  public async checkRoomStatus(
    roomId: string,
    platform?: SupportedPlatform
  ): Promise<boolean> {
    try {
      const roomInfo = await this.fetchRoomInfo(roomId, platform);
      return roomInfo.live_status;
    } catch (error) {
      console.error('Failed to check room status:', error);
      return false;
    }
  }
  
  // -------------------- 分类相关方法 --------------------
  
  // 获取直播分类列表
  public async fetchCategories(
    _parentId?: string,
    platform?: SupportedPlatform
  ): Promise<Category[]> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      // 使用统一的fetch_categories调用
      return await invoke<Category[]>('fetch_categories', {
        platform: this.getPlatformEnum(targetPlatform),
      });
    } catch (error) {
      console.error('Failed to fetch categories:', error);
      return [];
    }
  }
  
  // -------------------- 认证相关方法 --------------------
  
  // 获取认证URL
  public async getAuthUrl(
    platform?: SupportedPlatform
  ): Promise<string> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      return await invoke<string>('get_auth_url', {
        platform: this.getPlatformEnum(targetPlatform),
      });
    } catch (error) {
      console.error('Failed to get auth URL:', error);
      throw error;
    }
  }
  
  // 使用认证码登录
  public async loginWithCode(
    code: string,
    platform?: SupportedPlatform
  ): Promise<UserInfo> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      return await invoke<UserInfo>('login_with_code', {
        platform: this.getPlatformEnum(targetPlatform),
        code,
      });
    } catch (error) {
      console.error('Failed to login with code:', error);
      throw error;
    }
  }
  
  // 刷新认证令牌
  public async refreshToken(
    platform?: SupportedPlatform
  ): Promise<UserInfo> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      return await invoke<UserInfo>('refresh_token', {
        platform: this.getPlatformEnum(targetPlatform),
      });
    } catch (error) {
      console.error('Failed to refresh token:', error);
      throw error;
    }
  }
  
  // 获取当前用户信息
  public async getCurrentUser(
    platform?: SupportedPlatform
  ): Promise<UserInfo> {
    const targetPlatform = platform || this.currentPlatform;
    
    try {
      return await invoke<UserInfo>('get_current_user', {
        platform: this.getPlatformEnum(targetPlatform),
      });
    } catch (error) {
      console.error('Failed to get current user:', error);
      throw error;
    }
  }

  // 发送关注列表给后端
  public async sendFollowList(followList: FollowedStreamer[]): Promise<void> {
    try {
      await invoke<void>('send_follow_list', {
        followList: followList.map(streamer => ({
          platform: streamer.platform,
          room_id: streamer.roomId,
          streamer_id: streamer.id,
          nickname: streamer.nickname,
          avatar_url: streamer.avatarUrl
        }))
      });
    } catch (error) {
      console.error('Failed to send follow list:', error);
      throw error;
    }
  }
}

// 导出单例实例
export const platformApi = PlatformApiService.getInstance();