#define STRAT_UPDATE_MAGIC 0x5354524154555044ULL /* "STRATUPD" */
#define STRAT_UPDATE_MANIFEST_VERSION 1
#define STRAT_MAX_UPDATE_EXTENTS 512

typedef struct {
    uint64_t source_lba;
    uint64_t byte_len;
} StratUpdateExtent;

typedef struct {
    uint64_t magic;
    uint32_t version;
    uint32_t manifest_size;

    uint8_t target_slot;
    uint8_t reserved[7];

    uint8_t image_sha256[32];
    uint64_t image_size;

    uint32_t extent_count;
    StratUpdateExtent extents[STRAT_MAX_UPDATE_EXTENTS];

    uint32_t manifest_crc32;
} StratUpdateManifest;
