"""Do not report a successful delivery as successful saved-memory recall."""
import copy
import unittest

from daily_briefing_memory_smoke import require_recalled_memory


class RecallEvidence(unittest.TestCase):
    def setUp(self):
        self.saved = {'id': 'saved', 'content': 'Library discussion\n明晚七点在图书馆讨论。',
                      'scope': {'kind': 'conversation', 'conversation_id': 'qq'}}
        self.turn = {'state': 'completed', 'actions': [{
            'state': 'executed', 'tool': {'name': 'memory.search', 'arguments': {
                'scope': self.saved['scope'], 'text': '图书馆'}},
            'result': {'page': {'records': [self.saved]}},
        }], 'messages': [{'role': 'assistant', 'content': '\n'.join('> ' + line for line in self.saved['content'].splitlines())}]}

    def check(self, turn):
        require_recalled_memory(turn, self.saved, 'qq', '图书馆')

    def test_real_scoped_literal_hit_and_full_reply_pass(self):
        self.check(self.turn)

    def test_successful_empty_reply_is_not_a_hit(self):
        turn = copy.deepcopy(self.turn)
        turn['actions'][0]['result']['page']['records'] = []
        turn['messages'][0]['content'] = '当前会话没有可用的来源事项记忆。'
        with self.assertRaises(AssertionError):
            self.check(turn)

    def test_wrong_query_scope_record_or_result_body_is_not_a_hit(self):
        for change in ['query', 'scope', 'record', 'body', 'unexecuted']:
            with self.subTest(change=change):
                turn = copy.deepcopy(self.turn)
                action = turn['actions'][0]
                if change == 'query':
                    action['tool']['arguments']['text'] = None
                elif change == 'scope':
                    action['tool']['arguments']['scope']['conversation_id'] = 'other'
                elif change == 'record':
                    action['result']['page']['records'][0]['id'] = 'other'
                elif change == 'body':
                    action['result']['page']['records'][0]['content'] = 'wrong body'
                else:
                    action['state'] = 'proposed'
                with self.assertRaises(AssertionError):
                    self.check(turn)

    def test_right_tool_result_without_actual_answer_is_not_recall(self):
        turn = copy.deepcopy(self.turn)
        turn['messages'][0]['content'] = '当前会话没有可用的来源事项记忆。'
        with self.assertRaises(AssertionError):
            self.check(turn)


if __name__ == '__main__':
    unittest.main()
