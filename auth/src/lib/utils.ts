import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function maskEmail(email: string): string {
	if (!email) return '[no-email]';
	if (!email.includes('@')) return '***@***';
	const [username, domain] = email.split('@');
	return `${username.charAt(0)}***@${domain}`;
}

export function maskId(id: string): string {
	if (!id) return '[no-id]';
	return id.length > 6 ? `${id.substring(0, 3)}...${id.substring(id.length - 3)}` : '***';
}