import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import {PUBLIC_ENVIRONMENT} from "$env/static/public";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function inDev() {
	if (PUBLIC_ENVIRONMENT !== "dev" && PUBLIC_ENVIRONMENT !== "prod") {
		console.warn("Environment is not set to dev or prod");
		return;
	}

	return PUBLIC_ENVIRONMENT === "dev";
}

export function devLog(...args: any[]) {
	if (inDev() && console.log) {
		console.log(...args);
	}
}