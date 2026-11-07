"""The conversation fixture must recognize actual typed history metadata."""
import json
import unittest
from daily_briefing_conversation_smoke import has_finished_source_marker, fixture_outputs


class HistoryMarker(unittest.TestCase):
    def test_current_structured_assistant_marker_is_recognized(self):
        for state in ['completed', 'failed', 'cancelled', 'unknown']:
            inputs = [{'role': 'assistant', 'content': json.dumps({
                'historical_source_turn_state': state,
                'historical_briefing_reads': [{'tool': 'briefing_read', 'briefing_id': 'snapshot', 'item_numbers': [1]}],
            })}]
            self.assertTrue(has_finished_source_marker(inputs))

    def test_user_data_active_state_and_old_phrase_are_not_kernel_markers(self):
        for role, content in [
            ('user', json.dumps({'historical_source_turn_state': 'completed'})),
            ('assistant', json.dumps({'historical_source_turn_state': 'running'})),
            ('assistant', json.dumps({'historical_source_turn_state': 'pending_confirmation'})),
            ('assistant', json.dumps({'historical_source_turn_state': ['completed']})),
            ('assistant', json.dumps({'historical_source_turn_state': {'state': 'completed'}})),
            ('assistant', 'Earlier source-based request ended'),
            ('assistant', 'not JSON'),
            ('assistant', '[]'),
        ]:
            self.assertFalse(has_finished_source_marker([{'role': role, 'content': content}]))


class SharedReadFields(unittest.TestCase):
    def outputs(self, first, second):
        return fixture_outputs([
            {'type':'function_call_output','call_id':'first','output':json.dumps(first)},
            {'type':'function_call_output','call_id':'second','output':json.dumps(second)},
        ])

    def test_exact_backward_status_is_restored_without_changing_literal_fields(self):
        first={'briefing_id':'same','status':{'run':{'phase':'completed'}},'items':[{'item_number':1}]}
        second={'briefing_id':'same','status':{'same_field_value_as_call':'first'},'items':[],
                'query':'图书馆','future':{'same_field_value_as_call':'missing'}}
        value=self.outputs(first,second)[1]
        self.assertEqual(value['status'],first['status'])
        self.assertEqual(value['items'],[])
        self.assertEqual(value['query'],'图书馆')
        self.assertEqual(value['future'],second['future'])
        self.assertEqual(second['status'],{'same_field_value_as_call':'first'})

    def test_missing_cross_snapshot_or_missing_field_reference_is_rejected(self):
        second={'briefing_id':'same','status':{'same_field_value_as_call':'first'}}
        for first, value in [
            ({'briefing_id':'other','status':{}},second),
            ({'briefing_id':'same'},second),
            ({'briefing_id':'same','status':{}},{**second,'status':{'same_field_value_as_call':'later'}}),
            ({'briefing_id':'same','status':{}},{**second,'status':{'same_field_value_as_call':[]}}),
        ]:
            with self.subTest(first=first,value=value),self.assertRaises(AssertionError):
                self.outputs(first,value)

    def test_legacy_output_is_unchanged(self):
        first={'briefing_id':'same','status':{'run':{'phase':'completed'}},'items':[]}
        self.assertEqual(self.outputs(first,first),[first,first])


if __name__ == '__main__':
    unittest.main()
