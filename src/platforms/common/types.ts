// 支持的平台类型
export type SupportedPlatform = 'douyu' | 'bilibili' | 'douyin' | 'huya';

// 直播间基础信息
export interface CommonStreamer {
  room_id: string;
  title: string;
  nickname: string;
  avatar: string;
  room_cover: string;
  viewer_count_str: string;
  platform: SupportedPlatform;
  isLive: boolean;
}

// 关注的主播信息
export interface FollowedStreamer {
  id: string;
  platform: SupportedPlatform;
  nickname: string;
  avatarUrl: string;
  roomId: string;
  roomTitle?: string;
  followedAt?: number;
  currentRoomId?: string;
  isLive?: boolean;
  liveStatus?: LiveStatus;
}

// 分类相关类型
export interface CategorySelectedEvent {
  type: 'cate2';
  cate1Href: string;
  cate2Href: string;
  cate1Name: string;
  cate2Name: string;
  cate2Id?: string;
}

// 直播状态
export enum LiveStatus {
  LIVE = 'LIVE',
  REPLAY = 'REPLAY',
  OFFLINE = 'OFFLINE',
  UNKNOWN = 'UNKNOWN'
}

// 消息相关类型
export interface CommonMessage {
  id?: string;
  type: 'chat' | 'gift' | 'system' | 'enter' | 'other';
  content: string;
  sender: {
    nickname: string;
    uid?: string;
    level?: number | string;
    badgeName?: string;
    badgeLevel?: number;
  };
  timestamp?: number;
  color?: string;
  rawData?: Record<string, unknown>;
}
