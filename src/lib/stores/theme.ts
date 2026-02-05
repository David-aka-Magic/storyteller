import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type ThemeName = 'light' | 'dark' | 'midnight' | 'forest' | 'sunset' | 'ocean';

export interface Theme {
    name: ThemeName;
    label: string;
    colors: {
        // Backgrounds
        bgPrimary: string;
        bgSecondary: string;
        bgTertiary: string;
        bgChat: string;
        bgMessage: string;
        bgMessageUser: string;
        bgMessageAi: string;
        bgHover: string;
        bgActive: string;
        
        // Text
        textPrimary: string;
        textSecondary: string;
        textMuted: string;
        textInverse: string;
        
        // Borders
        borderPrimary: string;
        borderSecondary: string;
        borderActive: string;
        
        // Accents
        accentPrimary: string;
        accentSecondary: string;
        accentSuccess: string;
        accentDanger: string;
        accentWarning: string;
        
        // Shadows
        shadow: string;
    };
}

export const themes: Record<ThemeName, Theme> = {
    light: {
        name: 'light',
        label: '☀️ Light',
        colors: {
            bgPrimary: '#ffffff',
            bgSecondary: '#f4f4f9',
            bgTertiary: '#f9f9fc',
            bgChat: '#fafafa',
            bgMessage: '#ffffff',
            bgMessageUser: '#e3f2fd',
            bgMessageAi: '#ffffff',
            bgHover: '#e0e0e5',
            bgActive: '#4a9eff',
            textPrimary: '#333333',
            textSecondary: '#666666',
            textMuted: '#999999',
            textInverse: '#ffffff',
            borderPrimary: '#dddddd',
            borderSecondary: '#eeeeee',
            borderActive: '#2196f3',
            accentPrimary: '#007bff',
            accentSecondary: '#6c757d',
            accentSuccess: '#28a745',
            accentDanger: '#dc3545',
            accentWarning: '#ffc107',
            shadow: 'rgba(0, 0, 0, 0.1)',
        }
    },
    dark: {
        name: 'dark',
        label: '🌙 Dark',
        colors: {
            bgPrimary: '#1a1a2e',
            bgSecondary: '#16213e',
            bgTertiary: '#0f3460',
            bgChat: '#1a1a2e',
            bgMessage: '#16213e',
            bgMessageUser: '#0f3460',
            bgMessageAi: '#16213e',
            bgHover: '#0f3460',
            bgActive: '#e94560',
            textPrimary: '#eaeaea',
            textSecondary: '#b8b8b8',
            textMuted: '#888888',
            textInverse: '#1a1a2e',
            borderPrimary: '#0f3460',
            borderSecondary: '#16213e',
            borderActive: '#e94560',
            accentPrimary: '#e94560',
            accentSecondary: '#0f3460',
            accentSuccess: '#00d9a5',
            accentDanger: '#ff6b6b',
            accentWarning: '#feca57',
            shadow: 'rgba(0, 0, 0, 0.3)',
        }
    },
    midnight: {
        name: 'midnight',
        label: '🌌 Midnight',
        colors: {
            bgPrimary: '#0d1117',
            bgSecondary: '#161b22',
            bgTertiary: '#21262d',
            bgChat: '#0d1117',
            bgMessage: '#161b22',
            bgMessageUser: '#238636',
            bgMessageAi: '#21262d',
            bgHover: '#30363d',
            bgActive: '#58a6ff',
            textPrimary: '#c9d1d9',
            textSecondary: '#8b949e',
            textMuted: '#6e7681',
            textInverse: '#0d1117',
            borderPrimary: '#30363d',
            borderSecondary: '#21262d',
            borderActive: '#58a6ff',
            accentPrimary: '#58a6ff',
            accentSecondary: '#8b949e',
            accentSuccess: '#238636',
            accentDanger: '#f85149',
            accentWarning: '#d29922',
            shadow: 'rgba(0, 0, 0, 0.4)',
        }
    },
    forest: {
        name: 'forest',
        label: '🌲 Forest',
        colors: {
            bgPrimary: '#1a2f1a',
            bgSecondary: '#2d4a2d',
            bgTertiary: '#3d5c3d',
            bgChat: '#1a2f1a',
            bgMessage: '#2d4a2d',
            bgMessageUser: '#4a7c4a',
            bgMessageAi: '#2d4a2d',
            bgHover: '#3d5c3d',
            bgActive: '#6b8e6b',
            textPrimary: '#e8f5e9',
            textSecondary: '#a5d6a7',
            textMuted: '#81c784',
            textInverse: '#1a2f1a',
            borderPrimary: '#3d5c3d',
            borderSecondary: '#2d4a2d',
            borderActive: '#81c784',
            accentPrimary: '#66bb6a',
            accentSecondary: '#4a7c4a',
            accentSuccess: '#81c784',
            accentDanger: '#ef5350',
            accentWarning: '#ffb74d',
            shadow: 'rgba(0, 0, 0, 0.3)',
        }
    },
    sunset: {
        name: 'sunset',
        label: '🌅 Sunset',
        colors: {
            bgPrimary: '#2d1b2d',
            bgSecondary: '#4a2c4a',
            bgTertiary: '#5c3a5c',
            bgChat: '#2d1b2d',
            bgMessage: '#4a2c4a',
            bgMessageUser: '#ff6b6b',
            bgMessageAi: '#4a2c4a',
            bgHover: '#5c3a5c',
            bgActive: '#ff8e53',
            textPrimary: '#ffecd2',
            textSecondary: '#fcb69f',
            textMuted: '#d4a5a5',
            textInverse: '#2d1b2d',
            borderPrimary: '#5c3a5c',
            borderSecondary: '#4a2c4a',
            borderActive: '#ff8e53',
            accentPrimary: '#ff8e53',
            accentSecondary: '#ff6b6b',
            accentSuccess: '#a8e6cf',
            accentDanger: '#ff6b6b',
            accentWarning: '#ffeaa7',
            shadow: 'rgba(0, 0, 0, 0.3)',
        }
    },
    ocean: {
        name: 'ocean',
        label: '🌊 Ocean',
        colors: {
            bgPrimary: '#0a1628',
            bgSecondary: '#132337',
            bgTertiary: '#1c3347',
            bgChat: '#0a1628',
            bgMessage: '#132337',
            bgMessageUser: '#0077b6',
            bgMessageAi: '#1c3347',
            bgHover: '#1c3347',
            bgActive: '#00b4d8',
            textPrimary: '#caf0f8',
            textSecondary: '#90e0ef',
            textMuted: '#48cae4',
            textInverse: '#0a1628',
            borderPrimary: '#1c3347',
            borderSecondary: '#132337',
            borderActive: '#00b4d8',
            accentPrimary: '#00b4d8',
            accentSecondary: '#0077b6',
            accentSuccess: '#80ed99',
            accentDanger: '#ff6b6b',
            accentWarning: '#ffd166',
            shadow: 'rgba(0, 0, 0, 0.4)',
        }
    }
};

// Load saved theme or default to light
const savedTheme = browser ? (localStorage.getItem('app-theme') as ThemeName) || 'light' : 'light';

export const currentTheme = writable<ThemeName>(savedTheme);

// Save theme changes to localStorage
if (browser) {
    currentTheme.subscribe((theme) => {
        localStorage.setItem('app-theme', theme);
        applyTheme(theme);
    });
}

export function applyTheme(themeName: ThemeName) {
    if (!browser) return;
    
    const theme = themes[themeName];
    const root = document.documentElement;
    
    Object.entries(theme.colors).forEach(([key, value]) => {
        root.style.setProperty(`--${camelToKebab(key)}`, value);
    });
    
    root.setAttribute('data-theme', themeName);
}

function camelToKebab(str: string): string {
    return str.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
}

// Initialize theme on load
if (browser) {
    applyTheme(savedTheme);
}