import {z} from "zod";

const MAX_FIELD_LENGTH = 512;

// Base schemas for common types
export const clientIdSchema = z
    .string()
    .min(1, "Client ID is required")
    .max(
        MAX_FIELD_LENGTH,
        `Client ID must be ${MAX_FIELD_LENGTH} characters or less`
    );
export const scopeSchema = z
    .string()
    .max(
        MAX_FIELD_LENGTH,
        `Scope must be ${MAX_FIELD_LENGTH} characters or less`
    )
    .default("openid profile email");
export const grantTypeSchema = z
    .string()
    .min(1, "Grant type is required")
    .max(
        MAX_FIELD_LENGTH,
        `Grant type must be ${MAX_FIELD_LENGTH} characters or less`
    );
export const deviceCodeSchema = z
    .string()
    .min(1, "Device code is required")
    .max(
        MAX_FIELD_LENGTH,
        `Device code must be ${MAX_FIELD_LENGTH} characters or less`
    );
export const refreshTokenSchema = z
    .string()
    .min(1, "Refresh token is required")
    .max(
        MAX_FIELD_LENGTH,
        `Refresh token must be ${MAX_FIELD_LENGTH} characters or less`
    );

// Code challenge schemas
export const codeChallengeSchema = z
    .string()
    .min(43, "Code challenge must be at least 43 characters")
    .max(128, "Code challenge must be at most 128 characters")
    .regex(/^[A-Za-z0-9_-]+$/, "Code challenge must be base64url encoded");

export const codeChallengeMethodSchema = z.literal("S256");

export const codeVerifierSchema = z
    .string()
    .min(1, "Code verifier is required")
    .max(
        MAX_FIELD_LENGTH,
        `Code verifier must be ${MAX_FIELD_LENGTH} characters or less`
    );

// OAuth Device Authorization schema
export const deviceAuthorizeSchema = z
    .object({
        client_id: clientIdSchema.optional(),
        scope: scopeSchema,
        code_challenge: codeChallengeSchema,
        code_challenge_method: codeChallengeMethodSchema,
    })
    .refine(
        (data) => {
            // Both code_challenge and code_challenge_method must be present
            const hasChallenge = !!data.code_challenge;
            const hasMethod = !!data.code_challenge_method;
            return hasChallenge && hasMethod;
        },
        {
            message:
                "Both code_challenge and code_challenge_method are required",
        }
    );

// OAuth Device Token schema
export const deviceTokenSchema = z.object({
    grant_type: z.literal("urn:ietf:params:oauth:grant-type:device_code"),
    device_code: deviceCodeSchema,
    code_verifier: codeVerifierSchema,
    client_id: clientIdSchema.optional(),
});

// OAuth Refresh Token schema
export const refreshTokenRequestSchema = z.object({
    grant_type: z.literal("refresh_token"),
    refresh_token: refreshTokenSchema,
    client_id: clientIdSchema,
});

// Form validation helper
export const validateFormSize = (request: Request) => {
    const contentLength = request.headers.get("content-length");
    const MAX_FORM_SIZE = 1024; // 1KB max

    if (contentLength && parseInt(contentLength) > MAX_FORM_SIZE) {
        throw new Error("Request too large");
    }

    const contentType = request.headers.get("content-type");
    if (!contentType?.includes("application/x-www-form-urlencoded")) {
        throw new Error("Invalid content type");
    }
};

// Generic form data parser with Zod validation
export async function parseAndValidateForm<T>(
    request: Request,
    schema: z.ZodSchema<T>
): Promise<{success: true; data: T} | {success: false; error: string}> {
    try {
        // Validate form size and content type
        validateFormSize(request);

        // Parse form data
        const formData = await request.formData();

        // Convert FormData to plain object
        const formObj: Record<string, string> = {};
        for (const [key, value] of formData.entries()) {
            formObj[key] = value.toString();
        }

        // Validate with Zod
        const result = schema.safeParse(formObj);

        if (!result.success) {
            const firstError = result.error.issues[0];
            return {
                success: false,
                error: firstError.message,
            };
        }

        return {
            success: true,
            data: result.data,
        };
    } catch (error) {
        return {
            success: false,
            error:
                error instanceof Error
                    ? error.message
                    : "Invalid request format",
        };
    }
}
