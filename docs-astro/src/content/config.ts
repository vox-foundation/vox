import { defineCollection, z } from 'astro:content';
import { docsSchema } from '@astrojs/starlight/schema';

export const collections = {
  docs: defineCollection({
    schema: docsSchema({
      extend: z.object({
        category: z.string().optional(),
        status: z.string().optional(),
        training_eligible: z.boolean().optional(),
        sort_order: z.number().optional(),
        training_rationale: z.string().optional(),
        schema_type: z.string().optional(),
        last_updated: z.string().optional(),
      })
    }),
  }),
};
