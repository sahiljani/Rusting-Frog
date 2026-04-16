// Mint a short-lived HS256 JWT for testing against the API.
const crypto = require('crypto');

const secret = process.env.JWT_SECRET || 'dev-secret-change-in-production';

const header = { alg: 'HS256', typ: 'JWT' };
const now = Math.floor(Date.now() / 1000);
const payload = {
  tenant_id: process.env.TENANT_ID || 'test-tenant',
  user_id: process.env.USER_ID || 'test-user',
  scopes: ['crawl:read', 'crawl:write', 'export:read', 'config:write'],
  iat: now,
  exp: now + 3600,
};

const b64 = (obj) =>
  Buffer.from(JSON.stringify(obj)).toString('base64url');

const signingInput = `${b64(header)}.${b64(payload)}`;
const signature = crypto
  .createHmac('sha256', secret)
  .update(signingInput)
  .digest('base64url');

process.stdout.write(`${signingInput}.${signature}`);
