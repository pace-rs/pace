# Activity Reflection

## Time Range

Start: **{{ time_range_start }}**

End: **{{ time_range_end }}**

## Overview

Total Time Spent: **{{ total_time_spent }}**

Total Break Time: **{{ total_break_duration }}**

## Summary Groups By Category

| Category                                                                                                                                                 | Description       | Duration                           | Break Duration (Count)                                                           |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | ---------------------------------- | -------------------------------------------------------------------------------- |
| {% for category, summary_group in summary_groups_by_category -%} {%- for description, activity_group in summary_group.activity_groups_by_description -%} |                   |                                    |                                                                                  |
| {{ category }}                                                                                                                                           | {{ description }} | {{ summary_group.total_duration }} | {{ summary_group.total_break_duration }} ({{ summary_group.total_break_count }}) |
| {% endfor %}{% endfor %}                                                                                                                                 |                   |                                    |                                                                                  |
