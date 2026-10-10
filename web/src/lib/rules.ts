import { DEFAULT_POOL_DAILY_LIMIT_USDC } from '@/config/solana'

// What the family-vault program enforces, as the site describes it.
// Values mirror programs/family-vault; change them there first.
export const RULES = {
  maxBeneficiaries: 10,
  beneficiaryWaitHours: 48,
  dailyLimitUsdc: DEFAULT_POOL_DAILY_LIMIT_USDC,
  maxGuardians: 5,
  fastTrackSignatures: 2,
  ownerHandoverHours: 72,
} as const
