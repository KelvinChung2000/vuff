module m;
  initial begin
    if (ComState == GET_STOP_BIT &&
        ComTick == 1'b1 &&
        HexValue[4] == 1'b0) begin
      x = 1;
    end
  end
endmodule
// expected -----
module m;
  initial begin
    if (
      ComState == GET_STOP_BIT &&
      ComTick == 1'b1 &&
      HexValue[4] == 1'b0
    ) begin
      x = 1;
    end
  end
endmodule
