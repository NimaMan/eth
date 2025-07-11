### **Address Network Visualization: Conceptual Design & Encoding**

**1. Core Objective & Context**

The primary goal of the Address Network visualization is to explore the relationships and interactions centered around a specific target address. Unlike the *Token Network*, which focuses on flows related to a token contract, this view emphasizes the *connections between addresses themselves*.

This visualization directly addresses questions like:

*   Who interacts with the target address? (`@questions.md` Q10)
*   What are the roles of these interacting addresses (CEX, Staker, Contract, Scammer)? (`@questions.md` Q8)
*   Are there connections to known malicious actors or scam clusters? (`@questions.md` Q3, Q12, Q17)
*   Can we visually identify potentially risky associations? (`@questions.md` Q18)

**2. Core Concepts**

*   **Nodes:** Represent unique Ethereum addresses. The central node is the `target_address` provided by the user. Other nodes are addresses that have interacted with the target or are relevant within its N-hop neighborhood (definition of 'interaction' and 'neighborhood' depends on backend data generation).
*   **Edges:** Represent an observed interaction or relationship link between two addresses. The exact nature of the link (e.g., direct transaction, internal call, participation in the same trade involving a third party) is determined by the backend data generation logic, likely leveraging the `related_addresses` table from `@eth_db_data_models.py`.

**3. Node Encoding**

Nodes are styled based on their characteristics and relationships to provide immediate visual cues. We prioritize clarity for identifying key roles and potential risks.

| Feature        | Encoding Strategy                                                                 | Rationale                                                                                                 |
| :------------- | :-------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------- |
| **Color**      | Based on **primary category/role** of the address. Uses distinct, high-contrast colors. | Allows immediate identification of key players (Target, CEX, Scammer).                                    |
| **Shape**      | Based on **primary category/role**, reinforcing color information.                  | Provides a secondary visual cue, useful for accessibility and quick differentiation.                        |
| **Size**       | **User-selectable metric** (ETH Balance, Trade Count, Total Volume, Total Profit). Log-scaled. | Allows users to explore different dimensions of importance within the network.                              |
| **Border**     | Indicates **secondary risk factors** or important flags (e.g., interacted with scammer). | Adds crucial context without overriding the primary role color. Highlights potential indirect risks.      |
| **Label/Text** | Show truncated address by default. Full address/name on hover/tooltip.           | Keeps the graph clean while providing details on demand.                                                  |

**Detailed Node Encoding Table:**

| Node Type            | Identifier Criterion (from `Address` table or derived)                      | Color (Name)    | Hex Code  | Shape          | Border Color (If Applicable)        | Default Size Metric |
| :------------------- | :-------------------------------------------------------------------------- | :-------------- | :-------- | :------------- | :---------------------------------- | :------------------ |
| **Target Address**   | `address == target_address` (`is_target = true` flag from backend)         | Royal Blue      | `#4169E1` | `diamond`      | None (or Subtle Gold/White)         | Trade Count         |
| **Known Scammer**    | `is_scammer = true` OR `cluster_label == 'scammer'` OR `scam_ratio > threshold` | Dim Gray / Black| `#696969` | `triangle`     | Bright Red (`#FF0000`)              | Trade Count         |
| **CEX**              | `entity_category == 'CEX'` OR `is_cex = true` flag                          | Orange          | `#FFA500` | `square`       | None                                | ETH Balance         |
| **Staker / Staking** | `entity_category` contains 'Staker'/'Staking'                             | Medium Purple   | `#9370DB` | `star`         | None                                | ETH Balance         |
| **Known Token Contract** | `entity_category == 'Token'` OR (is_contract=true + known ERC20 heuristics) | Dark Turquoise  | `#00CED1` | `hexagon`      | None                                | Trade Count         |
| **Unknown Contract** | `is_contract = true` AND NOT identified as Token/CEX/etc.                   | Dark Gray       | `#A9A9A9` | `dot`          | None                                | Trade Count         |
| **Regular Address**  | Default EOA if none of the above apply                                      | Light Gray      | `#D3D3D3` | `dot`          | *(See Below)*                       | Trade Count         |
| ---                  | ---                                                                         | ---             | ---       | ---            | ---                                 | ---                 |
| _Risk Indicator:_    | **Interacted w/ Scammer** (`interacted_with_scammer = true` flag)          | _(Use Primary)_ | _(Use Primary)_ | _(Use Primary)_ | Orange (`#FFA500` or `#FF4500`) | _(Use Primary)_    |
| _Risk Indicator:_    | **High Scam Ratio** (`scam_ratio > threshold`, but not flagged as scammer) | _(Use Primary)_ | _(Use Primary)_ | _(Use Primary)_ | Light Red (`#FA8072`)             | _(Use Primary)_    |

**Notes on Node Encoding:**

*   **Identifier Criterion:** These rely heavily on data populated in the `eth_db.addresses` table (`@eth_db_data_models.py`). Flags like `is_target`, `is_scammer`, `is_cex`, and especially `interacted_with_scammer` need to be calculated by backend processes.

*   **`interacted_with_scammer` flag:** This is crucial. The backend needs to determine if a given address (that isn't itself a scammer) has a direct link in `related_addresses` to an address flagged as `is_scammer = true`.
*   **Border Color:** This is a key addition. A non-scammer node (e.g., Regular Address, CEX) could have a distinct border (e.g., Orange/Red) if it has interacted with a known scammer, alerting the user to potential risk propagation. The intensity/color of the border could even reflect the frequency or volume of scam interactions.
*   **Size Metric:** The UI should provide a dropdown allowing users to select the metric (`trade_count`, `total_denom_balance`, `total_volume`, `total_profit`) used for node sizing. Logarithmic scaling (`log(1 + value)`) should be applied to handle large value ranges.
*   **ETF/MEV Bot:** These categories from the token network are less central here but could be added if reliable `entity_category` labels exist or backend heuristics identify them. For now, they'd likely fall under 'Regular Address' or 'Unknown Contract'.

**4. Edge Encoding (Multiple Link Types)**

Edges represent relationships derived from transaction data. We will start with two primary types:

1.  **Fee Payer Link:** A **directed** edge from the transaction initiator (`from_address`, the fee payer) to *each other participant* in that same transaction. This highlights the funding source for the interaction.
2.  **Co-participant Link:** An **undirected** edge between *non-initiator participants* of the same transaction. This shows addresses clustered within the same transactional context, excluding the direct link to the fee payer (which is covered by the Fee Payer Link).

This approach provides distinct visual cues for different types of relationships observed within the same transaction set.

| Link Type             | Direction      | Color (Name)   | Hex Code  | Arrows | Width             | Opacity | Rationale                                                                                                   |
| :-------------------- | :------------- | :------------- | :-------- | :----- | :---------------- | :------ | :---------------------------------------------------------------------------------------------------------- |
| **Fee Payer -> Other**| Directed       | Bright Blue    | `#007BFF` | `to`   | `1.5px`           | `0.8`   | Clearly indicates the transaction initiator/funder. Directional arrow shows flow initiation.                |
| **Co-participant**    | Undirected     | Neutral Gray   | `#ADB5BD` | `none` | `1px` (Thinner)   | `0.6`   | Shows association between other parties in the transaction. Thinner and less opaque to reduce visual weight. |
| _Highlight (Hover)_   | (As Original)  | Bright Pink    | `#FF69B4` | (As Original) | `+0.5px`          | `1.0`   | Consistent highlight for easy tracing on hover.                                                               |

**Potential Future Edge Enhancements / Types:**

*   **Direct Transfer Edges:** Colored/weighted edges specifically showing ETH or Token transfers (requires querying transfer logs).
*   **Contract Interaction Edges:** Specific styles for calls between EOAs and contracts or between contracts.
*   **Frequency/Volume Weighting:** Varying edge width/opacity based on the *number* of co-participated transactions or total volume exchanged between addresses (if calculable).

**5. Interactivity**

*   **Hover Node:**
    *   Highlight the hovered node.
    *   Highlight its direct neighbors (connected by any edge type).
    *   Highlight the connecting edges using the `Highlight` color/style.
    *   Display a detailed tooltip for the hovered node.
*   **Click Node:**
    *   Smoothly pan/zoom the view to center on the clicked node.
    *   Keep the node highlighted.
    *   Display detailed information about the node in a side panel/table.
    *   Highlight the corresponding row in the data table.
*   **Click Background:** Reset highlights and zoom/pan.
*   **Layout Selection:** Dropdown for different physics/hierarchical layouts.
*   **Size Metric Selection:** Dropdown to select node sizing metric.

**6. Data Requirements & Backend Considerations**

The frontend (`address-network.js`) expects the API endpoint (`/api/addresses/network-graph`) to return graph data (`{nodes: [], links: []}`) where each node object has the defined fields, and the `links` array now contains objects specifying the type:

```json
// Link object structures
{
  "source": "0xfrom_address...",
  "target": "0xother_participant...",
  "type": "fee_payer" // Indicates a Fee Payer link
},
{
  "source": "0xparticipant_B...",
  "target": "0xparticipant_C...",
  "type": "co-participant" // Indicates a Co-participant link
}
// Optional future fields: "transaction_count": 5, "value": 1.23
```

**Backend Logic Needed (Updated):**

*   Determine the set of relevant transactions.
*   For each relevant transaction, identify the initiator (`from_address`) and other participants.
*   **Filter Participants:** Exclude addresses found in a predefined list of common/infrastructural addresses (e.g., WETH, known builders) from being processed as nodes or included in links, unless the address is the target itself.
*   **Generate Fee Payer Links:** For each transaction, create a directed edge (`type: 'fee_payer'`) from the `from_address` to every *other* unique, *non-excluded* participant in that transaction.
*   **Generate Co-participant Links:** For each transaction, identify all *non-excluded* participants *excluding* the `from_address`. Create undirected edges (`type: 'co-participant'`) between all unique pairs within this subset of other participants.
*   Fetch node data from `eth_db.addresses` for all unique, *non-excluded* participating addresses.
*   Calculate/retrieve necessary flags for each node (`is_target`, `is_contract`, `is_scammer`, `is_cex`, `interacted_with_scammer`).
*   Consolidate `entity_category` labels.
*   Return the `{nodes, links}` structure, ensuring links include the `type` field and only non-excluded nodes are present.

**7. Implementation Notes (Vis.js)**

*   Use Vis.js `nodes` properties (`color.background`, `color.border`, `shape`, `value`, `label`, `title`).
*   Implement node styling helper functions (`getNodeColor`, etc.) in JS.
*   **Edge Styling:** When mapping backend `links` to Vis.js `edges`, check the `link.type`. Set `color`, `arrows`, `width`, and `opacity` properties on the Vis.js edge object based on whether the type is `'fee_payer'` or `'co-participant'`. Use the values defined in the encoding table.
*   Configure interaction events.
*   Update the HTML legend to include an item for the Fee Payer link.

---